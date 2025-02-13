use chrono::{DateTime, Utc};
use sqlx::postgres::any::AnyConnectionBackend;
use uuid::Uuid;

use crate::{
    error::UnhandledError,
    metric_consts::{ISSUE_CREATED, ISSUE_REOPENED},
    types::{FingerprintedErrProps, OutputErrProps},
};

pub struct IssueFingerprintOverride {
    pub id: Uuid,
    pub team_id: i32,
    pub issue_id: Uuid,
    pub fingerprint: String,
    pub version: i64,
}

pub struct Issue {
    pub id: Uuid,
    pub team_id: i32,
    pub status: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Issue {
    pub fn new(team_id: i32, name: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            team_id,
            status: "active".to_string(), // TODO - we should at some point use an enum here
            name: Some(name),
            description: Some(description),
        }
    }

    pub async fn load_by_fingerprint<'c, E>(
        executor: E,
        team_id: i32,
        fingerprint: &str,
    ) -> Result<Option<Self>, UnhandledError>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let res = sqlx::query_as!(
            Issue,
            r#"
            SELECT i.id, i.team_id, i.status, i.name, i.description
            FROM posthog_errortrackingissue i
            JOIN posthog_errortrackingissuefingerprintv2 f ON i.id = f.issue_id
            WHERE f.team_id = $1 AND f.fingerprint = $2
            "#,
            team_id,
            fingerprint
        )
        .fetch_optional(executor)
        .await?;

        Ok(res)
    }

    pub async fn load<'c, E>(
        executor: E,
        team_id: i32,
        issue_id: Uuid,
    ) -> Result<Option<Self>, UnhandledError>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let res = sqlx::query_as!(
            Issue,
            r#"
            SELECT id, team_id, status, name, description FROM posthog_errortrackingissue
            WHERE team_id = $1 AND id = $2
            "#,
            team_id,
            issue_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(res)
    }

    pub async fn insert<'c, E>(&self, executor: E) -> Result<bool, UnhandledError>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let did_insert = sqlx::query_scalar!(
            r#"
            INSERT INTO posthog_errortrackingissue (id, team_id, status, name, description, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (id) DO NOTHING
            RETURNING (xmax = 0) AS was_inserted
            "#,
            self.id,
            self.team_id,
            self.status,
            self.name,
            self.description
        )
        .fetch_one(executor)
        .await?
        // TODO - I'm fairly sure the Option here is a bug in sqlx, so the unwrap will
        // never be hit, but nonetheless I'm not 100% sure the "no rows" case actually
        // means the insert was not done.
        .unwrap_or(false);

        if did_insert {
            metrics::counter!(ISSUE_CREATED).increment(1);
        }

        Ok(did_insert)
    }

    pub async fn maybe_reopen<'c, E>(&self, executor: E) -> Result<bool, UnhandledError>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let res = sqlx::query_scalar!(
            r#"
            UPDATE posthog_errortrackingissue
            SET status = 'active'
            WHERE id = $1 AND status != 'active'
            RETURNING id
            "#,
            self.id
        )
        .fetch_all(executor)
        .await?;

        let reopened = !res.is_empty();
        if reopened {
            metrics::counter!(ISSUE_REOPENED).increment(1);
        }

        Ok(reopened)
    }
}

impl IssueFingerprintOverride {
    pub async fn load<'c, E>(
        executor: E,
        team_id: i32,
        fingerprint: &str,
    ) -> Result<Option<Self>, UnhandledError>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let res = sqlx::query_as!(
            IssueFingerprintOverride,
            r#"
            SELECT id, team_id, issue_id, fingerprint, version FROM posthog_errortrackingissuefingerprintv2
            WHERE team_id = $1 AND fingerprint = $2
            "#,
            team_id,
            fingerprint
        ).fetch_optional(executor).await?;

        Ok(res)
    }

    pub async fn create_or_load<'c, E>(
        executor: E,
        team_id: i32,
        fingerprint: &str,
        issue: &Issue,
        first_seen: DateTime<Utc>,
    ) -> Result<Self, UnhandledError>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        // We do an "ON CONFLICT DO NOTHING" here because callers can compare the returned issue id
        // to the passed Issue, to see if the issue was actually inserted or not.
        let res = sqlx::query_as!(
            IssueFingerprintOverride,
            r#"
            INSERT INTO posthog_errortrackingissuefingerprintv2 (id, team_id, issue_id, fingerprint, version, first_seen, created_at)
            VALUES ($1, $2, $3, $4, 0, $5, NOW())
            ON CONFLICT (team_id, fingerprint) DO NOTHING
            RETURNING id, team_id, issue_id, fingerprint, version
            "#,
            Uuid::new_v4(),
            team_id,
            issue.id,
            fingerprint,
            first_seen
        ).fetch_one(executor).await?;

        Ok(res)
    }
}

pub async fn resolve_issue<'c, A>(
    con: A,
    team_id: i32,
    fingerprinted: FingerprintedErrProps,
    event_timestamp: DateTime<Utc>,
) -> Result<OutputErrProps, UnhandledError>
where
    A: sqlx::Acquire<'c, Database = sqlx::Postgres>,
{
    let mut conn = con.acquire().await?;

    // Fast path - just fetch the issue directly, and then reopen it if needed
    let existing_issue =
        Issue::load_by_fingerprint(&mut *conn, team_id, &fingerprinted.fingerprint).await?;
    if let Some(issue) = existing_issue {
        // TODO - we should use the bool here to determine if we need to notify a user
        // that the issue was reopened
        issue.maybe_reopen(&mut *conn).await?;
        return Ok(fingerprinted.to_output(issue.id));
    }

    // Slow path - insert a new issue, and then insert the fingerprint override, rolling
    // back the transaction if the override insert fails (since that indicates someone else
    // beat us to creating this new issue). Then, possibly reopen the issue.

    // UNWRAP: We never resolve an issue for an exception with no exception list
    let first = fingerprinted.exception_list.first().unwrap();
    let new_name = first.exception_type.clone();
    let new_description = first.exception_message.clone();

    // Start a transaction, so we can roll it back on override insert failure
    conn.begin().await?;
    // Insert a new issue
    let issue = Issue::new(team_id, new_name, new_description);
    // We don't actually care if we insert the issue here or not - conflicts aren't possible at
    // this stage.
    issue.insert(&mut *conn).await?;
    // Insert the fingerprint override
    let issue_override = IssueFingerprintOverride::create_or_load(
        &mut *conn,
        team_id,
        &fingerprinted.fingerprint,
        &issue,
        event_timestamp,
    )
    .await?;

    // If we actually inserted a new row for the issue override, commit the transaction,
    // saving both the issue and the override. Otherwise, rollback the transaction, and
    // use the retrieved issue override.
    let was_created = issue_override.issue_id == issue.id;
    if !was_created {
        conn.rollback().await?;
    } else {
        conn.commit().await?;
    }

    // This being None is /almost/ impossible, unless between the transaction above finishing and
    // this point, someone merged the issue and deleted the old one, but if that happens,
    // we don't care about this reopen failing (since this issue is irrelevant anyway). IT would be
    // more efficient to fetch the entire Issue struct above along with the fingerprint, but we're
    // in the slow path anyway, so one extra DB hit is not a big deal.
    if let Some(issue) = Issue::load(&mut *conn, team_id, issue_override.issue_id).await? {
        // TODO - we should use the bool here to determine if we need to notify a user
        // that the issue was reopened
        issue.maybe_reopen(&mut *conn).await?;
    }

    Ok(fingerprinted.to_output(issue_override.issue_id))
}

#[cfg(test)]
mod test {
    use crate::sanitize_string;

    #[test]
    fn it_replaces_null_characters() {
        let content = sanitize_string("\u{0000} is not valid JSON".to_string());
        assert_eq!(content, "� is not valid JSON");
    }
}
