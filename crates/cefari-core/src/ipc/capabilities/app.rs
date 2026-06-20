capability! {
    name: app,
    order: 10,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "App metadata can be shared, but lifecycle behavior differs by host.",
    commands: [
        AppQuit,
    ],
    results: [
        Empty,
    ],
    events: [
    ],
}
