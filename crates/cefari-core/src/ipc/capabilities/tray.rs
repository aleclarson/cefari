capability! {
    name: tray,
    order: 60,
    event_order: 35,
    support: desktopOnly,
    targets: [desktop],
    rationale: "Tray integration is a desktop OS concept with no mobile equivalent.",
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
