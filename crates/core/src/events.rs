//! Event System
//! 
//! Provides a pub/sub event bus for inter-component communication.

use parking_lot::RwLock;
use crossbeam_channel::{unbounded, Receiver, Sender};
use tracing::debug;

use crate::orchestrator::AppState;

/// Events that can be emitted throughout the IDE
#[derive(Debug, Clone)]
pub enum Event {
    /// Application state changed
    StateChanged(AppState),
    /// Configuration changed
    ConfigChanged,
    /// Workspace opened
    WorkspaceOpened,
    /// Workspace closed
    WorkspaceClosed,
    /// File opened
    FileOpened(std::path::PathBuf),
    /// File saved
    FileSaved(std::path::PathBuf),
    /// File modified
    FileModified(std::path::PathBuf),
    /// File closed
    FileClosed(std::path::PathBuf),
    /// Build started
    BuildStarted,
    /// Build progress update
    BuildProgress { stage: String, progress: f32 },
    /// Build completed
    BuildCompleted { success: bool, output_path: Option<std::path::PathBuf> },
    /// Emulator started
    EmulatorStarted { avd_name: String },
    /// Emulator stopped
    EmulatorStopped { avd_name: String },
    /// Log message
    Log { level: LogLevel, message: String },
    /// Error occurred
    Error { message: String, details: Option<String> },
    /// Android device connected
    DeviceConnected { device_id: String },
    /// Android device disconnected
    DeviceDisconnected { device_id: String },
    /// LSP notification
    LspNotification { server: String, notification: String },
    /// Manifest changed
    ManifestChanged,
    /// Permission toggled
    PermissionToggled { permission: String, enabled: bool },
    /// Theme changed
    ThemeChanged,
    /// Application shutdown
    Shutdown,
}

/// Log levels for log events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Subscriber handle for receiving events
#[derive(Clone)]
pub struct EventSubscription {
    receiver: Receiver<Event>,
}

impl EventSubscription {
    /// Receive the next event (blocking)
    pub fn recv(&self) -> Result<Event, crossbeam_channel::RecvError> {
        self.receiver.recv()
    }

    /// Try to receive an event (non-blocking)
    pub fn try_recv(&self) -> Result<Event, crossbeam_channel::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Get an iterator over events
    pub fn iter(&self) -> impl Iterator<Item = Event> + '_ {
        self.receiver.iter()
    }
}

/// Event bus for publish/subscribe pattern
pub struct EventBus {
    subscribers: RwLock<Vec<Sender<Event>>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> EventSubscription {
        let (sender, receiver) = unbounded();
        self.subscribers.write().push(sender);
        EventSubscription { receiver }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: Event) -> usize {
        let subscribers = self.subscribers.read();
        let mut delivered = 0;
        
        for sender in subscribers.iter() {
            if sender.send(event.clone()).is_ok() {
                delivered += 1;
            }
        }
        
        debug!("Event {:?} delivered to {} subscribers", event, delivered);
        delivered
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.read().len()
    }

    /// Clean up disconnected subscribers
    pub fn cleanup(&self) {
        let mut subscribers = self.subscribers.write();
        subscribers.retain(|s| !s.is_empty() || s.len() == 0);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Async event stream for tokio integration
pub struct AsyncEventStream {
    subscription: EventSubscription,
}

impl AsyncEventStream {
    pub fn new(event_bus: &EventBus) -> Self {
        Self {
            subscription: event_bus.subscribe(),
        }
    }

    /// Wait for the next event asynchronously
    pub async fn next(&self) -> Option<Event> {
        tokio::task::spawn_blocking({
            let subscription = self.subscription.clone();
            move || subscription.try_recv().ok()
        }).await.ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus() {
        let bus = EventBus::new();
        let sub1 = bus.subscribe();
        let sub2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        let delivered = bus.emit(Event::ConfigChanged);
        assert_eq!(delivered, 2);

        assert!(sub1.try_recv().is_ok());
        assert!(sub2.try_recv().is_ok());
    }
}
