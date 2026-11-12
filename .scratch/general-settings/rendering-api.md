# Rendering helpers for Settings

- `render::fonts::families() -> &'static [&'static str]` exposes sorted, unique names from the process font catalog, including bundled Inter.
- `render::fonts::search(query: &str, limit: usize) -> Vec<&'static str>` searches cached names without rescanning installed fonts. Call on search updates and keep the bounded result list in dialog state.
- `render::fonts::resolve(&FontSpec) -> iced::Font` resolves a stable family handle, preserves weight and falls back to Inter.
- `render::fonts::missing(&FontSpec) -> bool` identifies unavailable saved families.
- `render::slide::overflows(&Slide, &SlideTheme) -> bool` measures complete text at the 1920 reference width using the renderer's styles, wrapping, shaping and spacing. Call when material or draft settings change, and cache the result in dialog state.
- `render::slide::stage_with_guides(slide, theme, visibility)` takes the same arguments as `stage`; it adds safe-margin and content-padding outlines. `stage` and `thumbnail` keep their existing signatures and remain free of guides.

The optional projected section heading uses footer font, size and color. Headings and footers use iced's default 1.3 relative line height, and the gap between each text block is the footer size. Body text uses its configured line height. Output canvas format and all four safe margins come from the derived theme snapshot.
