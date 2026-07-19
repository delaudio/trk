use std::time::Instant;

use salieri_tui::NotificationKind;

use crate::{
    app_event::{AppEvent, NotificationLevel, NotificationRequest},
    App, Notification, NOTIFICATION_TTL,
};

impl App {
    fn notify(&mut self, kind: NotificationKind, message: impl Into<String>) {
        let level = match kind {
            NotificationKind::Info => NotificationLevel::Info,
            NotificationKind::Success => NotificationLevel::Success,
            NotificationKind::Warning => NotificationLevel::Warning,
            NotificationKind::Error => NotificationLevel::Error,
        };
        self.dispatch_event(AppEvent::Notification(NotificationRequest::new(
            level, message,
        )));
    }

    pub(crate) fn show_notification(&mut self, notification: NotificationRequest) {
        let kind = match notification.level {
            NotificationLevel::Info => NotificationKind::Info,
            NotificationLevel::Success => NotificationKind::Success,
            NotificationLevel::Warning => NotificationKind::Warning,
            NotificationLevel::Error => NotificationKind::Error,
        };
        self.notification = Some(Notification {
            kind,
            message: notification.message,
            expires_at: Instant::now() + NOTIFICATION_TTL,
        });
    }

    pub(crate) fn notify_info(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Info, message);
    }

    pub(crate) fn notify_success(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Success, message);
    }

    pub(crate) fn notify_warning(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Warning, message);
    }

    pub(crate) fn notify_error(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Error, message);
    }
}
