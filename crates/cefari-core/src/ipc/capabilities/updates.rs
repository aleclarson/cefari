capability! {
    name: updates,
    order: 40,
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
