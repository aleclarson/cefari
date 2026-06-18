capability! {
    name: service,
    order: 50,
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
