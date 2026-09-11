# NOTES_wgpu_ui — новый клиент (wgpu + winit)

## Структура
- `lib.rs` — ApplicationHandler (`App`), `Ctx` (split-borrow State для экранов), `dispatch`,
  FPS-счётчик (атомарный), `time_secs()`.
- `gfx.rs` — рендер: спрайт-батчер, пассы, RT, чтение пикселей. `sprite.wgsl` — один пайплайн
  (текстура × vertex color), альфа-блендинг SrcAlpha/OneMinusSrcAlpha, формат Bgra8Unorm.
- `camera.rs` — порт семантики macroquad `Camera2D::from_display_rect`:
  `screen = (world - target) * zoom * (viewport/2) + viewport/2`; GPU-матрица от viewport
  НЕ зависит (`gpu_uniform()` = [zx, zy, tx*zx, ty*zy]); Y-вниз всегда (h берётся по модулю).
- `text.rs` — fontdue + глифовый атлас 2048² (shelf packing), `measure`, `draw_text` (y = baseline),
  `draw_multiline` — точный порт macroquad split_text.
- `ui.rs` — immediate-mode UI: `UiCtx { gfx, text, input, skin, state, scale, cursor, origin }`,
  `InputState`, скины, дропдаун, скролл-группы. egui сознательно НЕ используется (планируется
  только для редактора карт позже).
- `screens.rs` — меню-экраны; `map_view.rs` — карта; `battle_view.rs` — бой; `bake.rs` — запечка
  карты/наплывов в RT; `assets.rs`, `files.rs`, `state.rs`.

## Кадр (важно для порядка)
1. `gfx.begin_frame()` — CPU-аккумуляция обнуляется.
2. Экраны открывают пассы: `gfx.begin_pass(Target::Screen|Target::Rt, clear: Option<[f32;4]>, camera)`.
   `clear=None` → LoadOp::Load (дослойка на ту же цель со сменой камеры — так HUD рисуется
   поверх карты). Пассы аккумулируются CPU-стороне; RenderPass никогда не хранится в структуре.
3. `gfx.end_frame()` — весь GPU-ворк одним сабмитом: для каждого пасса свои vertex/index буферы
   и свой camera uniform+bind group, затем submit, `queue.present(frame)`.
4. Ввод: события winit накапливаются между кадрами; `InputState::begin_frame` (начало кадра)
   НИЧЕГО не чистит; `InputState::end_frame` вызывается ПОСЛЕ dispatch — иначе кнопки мертвы.

## Правила заимствований в screens.rs (проверено компилятором)
- UiCtx держит `&mut ctx.gfx / ctx.text / ctx.widgets` — внутри `ui.window(|ui| ...)` рисовать
  только через `ui.gfx`/`ui.text`; доступ к игре — через захваченный локал `let game = &mut *ctx.game;`
  (дизъюнктные поля Ctx). Замыкания НЕ должны трогать `ctx` напрямую.
- Всё нужное из ctx извлекается в локалы ДО `UiCtx::new`; снапшоты данных (TexId, строки,
  display_unit-строки) — до UiCtx, замыкания читают локалы.
- `UiCtx::origin` — окно сдвигает курсор/рисование: None-виджеты кладутся по авто-layout курсору
  ОТНОСИТЕЛЬНО окна (порт macroquad Window). Скиссор `window()` — в физических пикселях (мир × scale).
- Мышь: `InputState.mouse` — физические пиксели; UI-мир = 1920×1080 → `UiCtx::mouse()` делит на
  `scale = viewport/(1920,1080)`. Бой рисуется в том же мире — `battle_view::is_clicked` конвертирует так же.
- Масштаб экрана: монитор 1920×1200 ≠ UI 1920×1080 — все проверки кликов ТОЛЬКО через конверсию.

## wgpu 30 API (факты, которые ломали сборку)
- `Instance::new(InstanceDescriptor::new_without_display_handle())` — по значению, без Default.
- `DeviceDescriptor` требует `experimental_features: wgpu::ExperimentalFeatures::disabled()`.
- `RequestAdapterOptions` требует `apply_limit_buckets: bool`.
- `SurfaceConfiguration` требует `color_space: SurfaceColorSpace::Auto`.
- `PipelineLayoutDescriptor`: `bind_group_layouts: &[Option<&BindGroupLayout>]`, вместо
  push_constant_ranges — `immediate_size: u32`.
- `RenderPipelineDescriptor`/`RenderPassDescriptor`: `multiview_mask: Option<NonZeroU32>`;
  `RenderPassColorAttachment` требует `depth_slice: Option<u32>`.
- `VertexState.buffers: &[Option<VertexBufferLayout>]`; `PipelineLayoutDescriptor.bind_group_layouts`
  тоже Option-wrapped; push_constant_ranges заменён на `immediate_size`.
- `surface.get_current_texture()` → enum `CurrentSurfaceTexture` { Success, Suboptimal, Timeout,
  Occluded, Outdated, Lost, Validation } — матчить, кадр можно пропускать.
- `device.poll(PollType::wait_indefinitely()) -> Result<PollStatus, PollError>`.
- `Queue::present(surface_texture)` — принимает SurfaceTexture ПО ЗНАЧЕНИЮ.
- `Instance::new(InstanceDescriptor::new_without_display_handle())` — по значению;
  поверхность: `instance.create_surface(window.clone())` где window: `Arc<Window>` → Surface<'static>.
- Трейт `wgpu::util::DeviceExt` нужно импортировать для `create_buffer_init`.
- `slice.get_mapped_range()` без аргументов, возвращает Result → `.expect(...)`.

## Игровой цикл
- `about_to_wait` → `request_redraw` + в конце `frame()` тоже (без этого X11 даёт ~0.75 FPS).
- `RedrawRequested → App::frame()`: input.begin_frame → ctx → dispatch → input.end_frame → gfx.end_frame.
- 60 FPS держится именно благодаря перезапросу из конца кадра (только about_to_wait = ~0.75 FPS).

## Открытые фиксы (сделать при возможности)
1. **Камера карты**: пер-пасс camera uniform в `end_frame` (см. NOTES_project). Симптом: карта
   статична при WASD/пан/зуме, всё остальное живое.
2. Android: `android_main` + `android-activity` (native-activity) в lib.rs готовы; проверить
   glue-конфликт с cargo-apk при первой реальной сборке.
3. Чистка: `dbg!` в dt_lib/units/unit.rs, eprintln-трейсы в battlefield.rs/screens.rs,
   мёртвый `pics/draws` в map_view.rs.
4. Текст мёртвых helper-ов screens.rs проверять при рефакторинге (частично уже реальные).