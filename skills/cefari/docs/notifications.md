# Notification Behavior

Cefari owns desktop notification permission checks and delivery through the
native runtime. App UI code should use `cefari/app` notification helpers rather
than calling platform notification APIs directly.

For TypeScript usage, see [Namespace APIs](typescript/namespaces.md).

## Startup Behavior

Cefari prepares notification support during desktop startup using the configured
app identifier and display name.

Startup does not send a notification and does not request notification
permission. Permission prompts must be triggered by an explicit user-visible
flow.

## Request Rules

Notification requests require a non-empty title. Optional text fields are
trimmed and rejected when blank.

The current TypeScript helper accepts:

```ts
await cefari.notifications.send({
  title: "Build complete",
  body: "The package is ready.",
});
```

The protocol also reserves notification permission and response events, but the
current desktop IPC dispatcher returns `unsupported` for notification commands
until notification IPC is wired end to end.

## Permission Behavior

Cefari checks OS notification permission before sending. If the OS reports that
notifications are denied or undetermined, Cefari treats the request as denied
instead of crashing the app.

Prompt for permission only from a user-visible action:

```ts
button.addEventListener("click", async () => {
  const permission = await cefari.notifications.requestPermission();
  console.log(permission.allowed);
});
```

## Platform Notes

- macOS requires a real app bundle identifier before native notifications can be
  delivered.
- macOS permission prompts must happen from explicit user-visible flows.
- Windows toast setup uses the configured app identifier.
- Linux and other XDG desktop targets depend on the notification service
  available in the user's desktop session.
- Notification actions, replies, and categories are not a stable cross-platform
  Cefari app contract yet.
