# Cefari CSS Contract

Cefari provides a tiny built-in CSS contract for custom titlebars:

```css
.cefari-drag {
  -webkit-app-region: drag;
}

.cefari-no-drag,
.cefari-drag button,
.cefari-drag input,
.cefari-drag textarea,
.cefari-drag select,
.cefari-drag a {
  -webkit-app-region: no-drag;
}
```

Drag regions are opt-in. Cefari does not automatically make headers, navbars, titlebars, or arbitrary app chrome draggable.

Use `cefari-drag` on the exact region that should move the window. Use `cefari-no-drag` on interactive descendants that must keep normal pointer behavior. Buttons, inputs, textareas, selects, and links inside a `cefari-drag` region default to no-drag.

The stylesheet is installed by the trusted main-frame bridge bootstrap and prepended as `#cefari-default-styles`, so app CSS can override it when needed.

See [Develop Locally](guides/development.md) for template usage and [Native Capabilities](guides/native-capabilities.md) for the broader native surface.
