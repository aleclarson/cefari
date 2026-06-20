capability! {
    name: workers,
    order: 110,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "Worker-like execution may exist across hosts, but lifecycle limits differ.",
    commands: [
        Worker(WorkerCommand),
    ],
    results: [
        Worker(WorkerResult),
    ],
    events: [
        Worker(WorkerEvent),
    ],
}
