capability! {
    name: workers,
    order: 110,
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
