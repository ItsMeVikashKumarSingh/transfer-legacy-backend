BEGIN;

-- User In-App Notifications
CREATE TABLE IF NOT EXISTS notify.user_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    notification_type TEXT NOT NULL DEFAULT 'system',
    icon TEXT NOT NULL DEFAULT 'Bell',
    action_url TEXT NULL,
    read BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_user_notifications_user_unread 
    ON notify.user_notifications(user_id, read, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_user_notifications_user_created 
    ON notify.user_notifications(user_id, created_at DESC);

-- User Notification & Signal Preferences
CREATE TABLE IF NOT EXISTS notify.notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    email_enabled BOOLEAN NOT NULL DEFAULT true,
    push_enabled BOOLEAN NOT NULL DEFAULT true,
    in_app_enabled BOOLEAN NOT NULL DEFAULT true,
    security_alerts BOOLEAN NOT NULL DEFAULT true,
    heartbeat_reminders BOOLEAN NOT NULL DEFAULT true,
    guardian_signals BOOLEAN NOT NULL DEFAULT true,
    vault_updates BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMIT;
