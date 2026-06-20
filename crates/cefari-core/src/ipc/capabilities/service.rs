capability! {
    name: service,
    order: 50,
    support: desktopOnly,
    targets: [desktop],
    rationale: "Local daemon service management is a desktop runtime capability.",
    commands: [
        ServiceStatus,
    ],
    results: [
        ServiceStatus(ServiceStatusResult),
    ],
    events: [
        ServiceStatusChanged(ServiceStatusResult),
    ],
}
