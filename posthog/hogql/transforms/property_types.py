from typing import Dict, Set

from posthog.hogql import ast
from posthog.hogql.context import HogQLContext
from posthog.hogql.database.models import DateTimeDatabaseField
from posthog.hogql.escape_sql import escape_hogql_identifier
from posthog.hogql.parser import parse_expr
from posthog.hogql.visitor import CloningVisitor, TraversingVisitor
from posthog.schema import HogQLNotice


def resolve_property_types(node: ast.Expr, context: HogQLContext = None) -> ast.Expr:
    from posthog.models import PropertyDefinition

    # find all properties
    property_finder = PropertyFinder()
    property_finder.visit(node)

    # fetch them
    event_property_values = (
        PropertyDefinition.objects.filter(
            name__in=property_finder.event_properties,
            team_id=context.team_id,
            type__in=[None, PropertyDefinition.Type.EVENT],
        ).values_list("name", "property_type")
        if property_finder.event_properties
        else []
    )
    event_properties = {name: property_type for name, property_type in event_property_values if property_type}

    person_property_values = (
        PropertyDefinition.objects.filter(
            name__in=property_finder.person_properties,
            team_id=context.team_id,
            type=PropertyDefinition.Type.PERSON,
        ).values_list("name", "property_type")
        if property_finder.person_properties
        else []
    )
    person_properties = {name: property_type for name, property_type in person_property_values if property_type}

    # swap them out
    if len(event_properties) == 0 and len(person_properties) == 0 and not property_finder.found_timestamps:
        return node

    timezone = context.database.get_timezone() if context and context.database else "UTC"
    property_swapper = PropertySwapper(
        timezone=timezone, event_properties=event_properties, person_properties=person_properties, context=context
    )
    return property_swapper.visit(node)


class PropertyFinder(TraversingVisitor):
    def __init__(self):
        super().__init__()
        self.person_properties: Set[str] = set()
        self.event_properties: Set[str] = set()
        self.found_timestamps = False

    def visit_property_type(self, node: ast.PropertyType):
        if node.field_type.name == "properties" and len(node.chain) == 1:
            if isinstance(node.field_type.table_type, ast.BaseTableType):
                table = node.field_type.table_type.resolve_database_table().to_printed_hogql()
                if table == "persons" or table == "raw_persons":
                    self.person_properties.add(node.chain[0])
                if table == "events":
                    if (
                        isinstance(node.field_type.table_type, ast.VirtualTableType)
                        and node.field_type.table_type.field == "poe"
                    ):
                        self.person_properties.add(node.chain[0])
                    else:
                        self.event_properties.add(node.chain[0])

    def visit_field(self, node: ast.Field):
        super().visit_field(node)
        if isinstance(node.type, ast.FieldType) and isinstance(
            node.type.resolve_database_field(), DateTimeDatabaseField
        ):
            self.found_timestamps = True


class PropertySwapper(CloningVisitor):
    def __init__(
        self, timezone: str, event_properties: Dict[str, str], person_properties: Dict[str, str], context: HogQLContext
    ):
        super().__init__(clear_types=False)
        self.timezone = timezone
        self.event_properties = event_properties
        self.person_properties = person_properties
        self.context = context

    def visit_field(self, node: ast.Field):
        if isinstance(node.type, ast.FieldType):
            if isinstance(node.type.resolve_database_field(), DateTimeDatabaseField):
                return ast.Call(name="toTimeZone", args=[node, ast.Constant(value=self.timezone)])

        type = node.type
        if isinstance(type, ast.PropertyType) and type.field_type.name == "properties" and len(type.chain) == 1:
            if (
                isinstance(type.field_type.table_type, ast.VirtualTableType)
                and type.field_type.table_type.field == "poe"
            ):
                if type.chain[0] in self.person_properties:
                    return self._add_type_to_string_field(node, self.person_properties[type.chain[0]])
            elif isinstance(type.field_type.table_type, ast.BaseTableType):
                table = type.field_type.table_type.resolve_database_table().to_printed_hogql()
                if table == "persons" or table == "raw_persons":
                    if type.chain[0] in self.person_properties:
                        return self._add_type_to_string_field(node, self.person_properties[type.chain[0]])
                if table == "events":
                    if type.chain[0] in self.event_properties:
                        return self._add_type_to_string_field(node, self.event_properties[type.chain[0]])
        if isinstance(type, ast.PropertyType) and type.field_type.name == "person_properties" and len(type.chain) == 1:
            if isinstance(type.field_type.table_type, ast.BaseTableType):
                table = type.field_type.table_type.resolve_database_table().to_printed_hogql()
                if table == "events":
                    if type.chain[0] in self.person_properties:
                        return self._add_type_to_string_field(node, self.person_properties[type.chain[0]])

        return node

    def _add_notice(self, node: ast.Field, message: str):
        # Only highlight the last part of the chain
        start = max(node.start, node.end - len(escape_hogql_identifier(node.chain[-1])))
        self.context.notices.append(
            HogQLNotice(
                start=start,
                end=node.end,
                message=message,
            )
        )

    def _add_type_to_string_field(self, node: ast.Field, type: str):
        if type == "DateTime":
            self._add_notice(node=node, message=f"Property '{node.chain[-1]}' is of type 'DateTime'")
            return ast.Call(name="toDateTime", args=[node])
        if type == "Numeric":
            self._add_notice(node=node, message=f"Property '{node.chain[-1]}' is of type 'Float'")
            return ast.Call(name="toFloat", args=[node])
        if type == "Boolean":
            self._add_notice(node=node, message=f"Property '{node.chain[-1]}' is of type 'Boolean'")
            return parse_expr("{node} = 'true'", {"node": node})
        self._add_notice(node=node, message=f"Property '{node.chain[-1]}' is of type 'String'")
        return node
