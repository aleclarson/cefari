capability! {
    name: updates,
    order: 40,
    support: desktopOnly,
    targets: [desktop],
    rationale: "The desktop updater contract should not be reused for app-store mobile updates.",
    commands: [
        UpdateState,
        UpdateCheck,
        UpdateApply(UpdateApplyRequest),
        UpdateRestart,
    ],
    results: [
        UpdateState(UpdateStateResult),
        UpdateCheck(UpdateCheckResult),
        UpdateApply(UpdateApplyResult),
    ],
    events: [
        UpdateStateChanged(UpdateStateResult),
    ],
}
