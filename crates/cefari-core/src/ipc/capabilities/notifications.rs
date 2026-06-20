capability! {
    name: notifications,
    order: 80,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "Notification intent is shared, but permissions and delivery behavior differ by host.",
    commands: [
        Notification(NotificationCommand),
    ],
    results: [
        Notification(NotificationResult),
    ],
    events: [
        Notification(NotificationEvent),
    ],
}
