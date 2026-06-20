capability! {
    name: logs,
    order: 75,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "Log capture is shared at the API level, but persistence and export routing differ by host.",
    commands: [
        Log(LogRequest),
    ],
    results: [
    ],
    events: [
    ],
}
