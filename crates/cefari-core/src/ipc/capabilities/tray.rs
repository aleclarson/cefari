capability! {
    name: tray,
    order: 60,
    event_order: 35,
    commands: [
        TrayRestoreWindow,
    ],
    results: [
        Tray(TrayResult),
    ],
    events: [
        TrayRestoreWindow,
    ],
}
