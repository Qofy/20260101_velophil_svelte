1. Content JSON + Data Fetch
- Extract text hierarchy and metadata from velophil_new_image_2026.png.
- Define article.json with meta, sections, parallax scene specs, and
sidebar data.
- Hook up onMount fetch + store to hydrate the main route.
2. Component Architecture
- Replace the current App.svelte markup with a Routify layout that
renders Nav, Breadcrumbs, Hero, Article sections, Sidebars, Footer.
- Create reusable components for rich blocks (text/image pairs, quotes,
CTA buttons, consent video placeholder, gallery).
- Drop any unused WASM demo UI while keeping the initialization
scaffolding minimal so WASM bits remain available if needed.
3. Parallax Scenes
- Use the scene_1 and scene_2 slices: stack each slice as a layer with
position: sticky wrappers.
- Build a small useParallax action to throttle scroll events and
translate each layer at different speeds so the overlap mimics the
live site.
- Ensure z-index ordering keeps the fixed nav on top while allowing the
layers to overlap other content sections.
4. Responsive/Lazy Images
- Create a helper for srcset + sizes using the provided PNGs (and
placeholders for still-missing assets).
- Apply loading="lazy", decoding="async", plus aspect-ratio containers
to avoid layout shift.
5. Styling + Verification
- Port fonts/spacing from the reference screenshot (likely a serif
headline + sans paragraph).
- Implement CSS variables for palette to ensure consistent colors across
sections.
- Run bun run dev locally (even if socket restricted) and at least npm
run build to validate compilation.
