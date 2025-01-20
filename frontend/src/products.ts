// This file is autogenerated

export const productScenes: Record<string, any> = {
    EarlyAccessFeatures: (): any => import('../../products/early_access_features/frontend/EarlyAccessFeatures'),
    EarlyAccessFeature: (): any => import('../../products/early_access_features/frontend/EarlyAccessFeature'),
    LLMObservability: (): any => import('../../products/llm_observability/frontend/LLMObservabilityScene'),
    LLMObservabilityTrace: (): any => import('../../products/llm_observability/frontend/LLMObservabilityTraceScene'),
    MessagingBroadcasts: (): any => import('../../products/messaging/frontend/Broadcasts'),
    MessagingProviders: (): any => import('../../products/messaging/frontend/Providers'),
}

export const productRoutes: Record<string, [string, string]> = {
    '/early_access_features': ['EarlyAccessFeatures', 'earlyAccessFeatures'],
    '/early_access_features/:id': ['EarlyAccessFeature', 'earlyAccessFeature'],
    '/llm-observability': ['LLMObservability', 'llmObservability'],
    '/llm-observability/dashboard': ['LLMObservability', 'llmObservabilityDashboard'],
    '/llm-observability/generations': ['LLMObservability', 'llmObservabilityGenerations'],
    '/llm-observability/traces': ['LLMObservability', 'llmObservabilityTraces'],
    '/llm-observability/traces/:id': ['LLMObservabilityTrace', 'llmObservability'],
    '/messaging/providers': ['MessagingProviders', 'messagingProviders'],
    '/messaging/providers/:id': ['MessagingProviders', 'messagingProvider'],
    '/messaging/providers/new': ['MessagingProviders', 'messagingProviderNew'],
    '/messaging/providers/new/*': ['MessagingProviders', 'messagingProviderNew'],
    '/messaging/broadcasts': ['MessagingBroadcasts', 'messagingBroadcasts'],
    '/messaging/broadcasts/:id': ['MessagingBroadcasts', 'messagingBroadcast'],
    '/messaging/broadcasts/new': ['MessagingBroadcasts', 'messagingBroadcastNew'],
}

export const productRedirects: Record<string, string> = { '/messaging': '/messaging/broadcasts' }

export const productConfiguration: Record<string, any> = {
    EarlyAccessFeatures: {
        name: 'Early Access Features',
        projectBased: true,
        defaultDocsPath: '/docs/feature-flags/early-access-feature-management',
        activityScope: 'EarlyAccessFeature',
    },
    EarlyAccessFeature: {
        name: 'Early Access Features',
        projectBased: true,
        defaultDocsPath: '/docs/feature-flags/early-access-feature-management',
        activityScope: 'EarlyAccessFeature',
    },
    LLMObservability: {
        name: 'LLM observability',
        projectBased: true,
        activityScope: 'LLMObservability',
        layout: 'app-container',
        defaultDocsPath: '/docs/ai-engineering/observability',
    },
    LLMObservabilityTrace: {
        name: 'LLM observability trace',
        projectBased: true,
        activityScope: 'LLMObservability',
        layout: 'app-container',
        defaultDocsPath: '/docs/ai-engineering/observability',
    },
    MessagingBroadcasts: { name: 'Messaging', projectBased: true },
    MessagingProviders: { name: 'Messaging', projectBased: true },
}

export const productUrls = {
    earlyAccessFeatures: (): string => '/early_access_features',
    earlyAccessFeature: (id: string): string => `/early_access_features/${id}`,
    llmObservability: (tab?: 'dashboard' | 'traces' | 'generations'): string =>
        `/llm-observability${tab !== 'dashboard' ? '/' + tab : ''}`,
    llmObservabilityTrace: (id: string, eventId?: string): string =>
        `/llm-observability/traces/${id}${eventId ? `?event=${eventId}` : ''}`,
    messagingBroadcasts: (): string => '/messaging/broadcasts',
    messagingBroadcast: (id?: string): string => `/messaging/broadcasts/${id}`,
    messagingBroadcastNew: (): string => '/messaging/broadcasts/new',
    messagingProviders: (): string => '/messaging/providers',
    messagingProvider: (id?: string): string => `/messaging/providers/${id}`,
    messagingProviderNew: (template?: string): string => '/messaging/providers/new' + (template ? `/${template}` : ''),
}
