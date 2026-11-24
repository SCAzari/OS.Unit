//! Display and input backend: DRM/KMS through a libseat session, OpenGL ES rendering on GBM, and
//! libinput for keyboards, pointers and touchscreens.
//!
//! This first version drives the first connected display of the primary GPU. On core.unit that is
//! the integrated graphics (Software Architecture r1.16 §4); in emulation it is the virtual GPU,
//! rendered in software by Mesa.

use std::{path::PathBuf, time::Duration};

use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            compositor::{DrmCompositor, FrameFlags, PrimaryPlaneElement},
            exporter::gbm::GbmFramebufferExporter,
            DrmDevice, DrmDeviceFd, DrmEvent, DrmNode,
        },
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            element::{
                render_elements,
                solid::SolidColorRenderElement,
                Id, Kind,
            },
            element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
            utils::CommitCounter,
            Color32F, ImportDma,
        },
        session::{libseat::LibSeatSession, Event as SessionEvent, Session},
        udev::{all_gpus, primary_gpu},
    },
    desktop::space::SpaceRenderElements,
    output::{Mode as WlMode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop, LoopHandle,
        },
        drm::{
            control::{connector, crtc, Device as ControlDevice, ModeTypeFlags},
            Device as BasicDevice,
        },
        input::Libinput,
        rustix::fs::OFlags,
    },
    utils::{DeviceFd, Physical, Rectangle, Transform},
    wayland::compositor::SurfaceData,
};

use crate::{
    layout::{self, DisplayConfig, LayoutClass},
    state::UnitState,
};

/// Display background before the Shell UI draws: Daylight surface colour #F4F2EE
/// (Visual Design Specification).
const BACKGROUND: [f32; 4] = [0.957, 0.949, 0.933, 1.0];
/// Pointer: Daylight default accent #11756B.
const POINTER_COLOUR: [f32; 4] = [0.067, 0.459, 0.420, 1.0];
const POINTER_SIZE: i32 = 14;
/// Colour formats tried for the display, 8 bits per channel first.
const COLOR_FORMATS: &[Fourcc] = &[Fourcc::Argb8888, Fourcc::Xrgb8888, Fourcc::Abgr8888];
/// Redraw check while nothing has changed (about 60 per second).
const IDLE_REDRAW: Duration = Duration::from_millis(16);
/// A queued frame whose vertical blank has not arrived after this long is treated as lost, so
/// the display cannot stay frozen. Generous, because the emulated display can be slow.
const FRAME_TIMEOUT: Duration = Duration::from_secs(1);

type GbmDrmCompositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

render_elements! {
    pub UnitRenderElements<=GlesRenderer>;
    Space=SpaceRenderElements<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>,
    Pointer=SolidColorRenderElement,
}

pub struct UdevData {
    pub session: LibSeatSession,
    drm: DrmDevice,
    renderer: GlesRenderer,
    compositor: GbmDrmCompositor,
    output: Output,
    pointer_id: Id,
    /// A frame has been queued and its vertical blank has not arrived yet.
    frame_pending: bool,
    /// Number of the last queued frame, so the frame watchdog acts only on the frame it was set for.
    frame_seq: u64,
    /// A redraw timer is already scheduled.
    redraw_scheduled: bool,
    /// How frames may use the display's hardware planes (none on the emulated GPU).
    frame_flags: FrameFlags,
    /// Wait for drawing to finish before queueing a frame when the display path needs it. Always
    /// on; the kernel option unit.render_wait=0 turns it off for the UNIT-OS-ISS-0020 A/B test.
    render_wait: bool,
    handle: LoopHandle<'static, UnitState>,
}

pub fn init(
    event_loop: &mut EventLoop<'static, UnitState>,
    state: &mut UnitState,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = event_loop.handle();

    // Session: logind when the compositor runs in a user session, otherwise libseat's fallback.
    let (mut session, session_notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    tracing::info!("session on {}", seat_name);

    // Primary GPU and its first connected display.
    let gpu_path: PathBuf = match primary_gpu(&seat_name)? {
        Some(path) => path,
        None => all_gpus(&seat_name)?
            .into_iter()
            .next()
            .ok_or("no GPU found")?,
    };
    tracing::info!("using GPU {}", gpu_path.display());
    let fd = session.open(
        &gpu_path,
        OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
    )?;
    let fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let node = DrmNode::from_file(fd.clone()).ok();
    let (mut drm, drm_notifier) = DrmDevice::new(fd.clone(), true)?;
    let mut frame_flags = FrameFlags::DEFAULT;
    let render_wait = !crate::state::kernel_option("unit.render_wait=0");
    if !render_wait {
        tracing::warn!("render wait off (unit.render_wait=0): diagnostic test only");
    }
    // The emulated GPU has no 3D acceleration in the test machines: render in software there.
    // On real hardware (for example Intel Xe graphics) Mesa's hardware driver is used.
    if let Ok(driver) = fd.get_driver() {
        let name = driver.name().to_string_lossy().into_owned();
        tracing::info!("GPU driver {}", name);
        if name == "virtio_gpu" && std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_none() {
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            tracing::info!("emulated GPU: software rendering");
        }
        if name == "virtio_gpu" {
            // The emulated display's screenshots do not include hardware planes such as the cursor
            // plane: draw everything into one buffer, so the screenshots show the whole picture.
            frame_flags = FrameFlags::empty();
            tracing::info!("emulated GPU: no hardware planes; pointer drawn into the picture");
        }
    }
    let gbm = GbmDevice::new(fd.clone())?;

    // Safety: the GBM device outlives the display, which is kept by the renderer.
    let egl_display = unsafe { EGLDisplay::new(gbm.clone())? };
    let context = EGLContext::new(&egl_display)?;
    // Safety: the context is current only on this thread.
    let renderer = unsafe { GlesRenderer::new(context)? };

    let (connector_info, crtc, mode) = find_display(&fd).ok_or("no connected display found")?;
    let output_name = format!("{}-{}", connector_info.interface().as_str(), connector_info.interface_id());
    let (w_mm, h_mm) = connector_info.size().unwrap_or((0, 0));
    tracing::info!(
        "display {}: {}x{} at {} Hz",
        output_name,
        mode.size().0,
        mode.size().1,
        mode.vrefresh()
    );

    let output = Output::new(
        output_name,
        PhysicalProperties {
            size: (w_mm as i32, h_mm as i32).into(),
            subpixel: Subpixel::Unknown,
            make: "Unit".into(),
            model: "Display".into(),
        },
    );
    // Density and layout class (Requirement OS-52 - adaptive interface; Interface Design §5.1).
    let config = DisplayConfig::load();
    let connector_type = connector_info.interface().as_str();
    let kind = config.screen_kind(connector_type);
    let (px_w, px_h) = (mode.size().0 as i32, mode.size().1 as i32);
    let density = layout::density(px_w, px_h, (w_mm, h_mm), kind, config.density_factor);
    let scale = density.factor * config.display_scaling;
    let width_points = layout::layout_width(px_w, density.factor, config.display_scaling);
    let class = LayoutClass::for_width(width_points);
    tracing::info!(
        "layout: {:?} screen, density factor {:.3} ({:?}), scaling {:.2}, {} points wide: {} class",
        kind,
        density.factor,
        density.source,
        config.display_scaling,
        width_points,
        class
    );
    if class != LayoutClass::Compact {
        tracing::info!(
            "layout: the {} layout comes in a later increment; using the compact layout",
            class
        );
    }
    state.layout_class = class;

    let wl_mode = WlMode::from(mode);
    output.change_current_state(
        Some(wl_mode),
        Some(Transform::Normal),
        Some(Scale::Fractional(scale)),
        Some((0, 0).into()),
    );
    output.set_preferred(wl_mode);
    let _global = output.create_global::<UnitState>(&state.display_handle);
    state.space.map_output(&output, (0, 0));

    let surface = drm.create_surface(crtc, mode, &[connector_info.handle()])?;
    let allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT);
    let exporter = GbmFramebufferExporter::new(gbm.clone(), node);
    let renderer_formats = renderer.dmabuf_formats();
    let compositor = GbmDrmCompositor::new(
        &output,
        surface,
        None,
        allocator,
        exporter,
        COLOR_FORMATS.iter().copied(),
        renderer_formats,
        drm.cursor_size(),
        Some(gbm),
    )
    .map_err(|err| format!("display set-up failed: {:?}", err))?;

    // Vertical blank: the queued frame is on screen; draw the next one.
    handle.insert_source(drm_notifier, |event, _, state| match event {
        DrmEvent::VBlank(_crtc) => {
            if let Some(backend) = state.backend.as_mut() {
                if let Err(err) = backend.compositor.frame_submitted() {
                    tracing::warn!("frame submission: {:?}", err);
                }
                backend.frame_pending = false;
            }
            state.render();
        }
        // A vertical blank lost with the error is recovered by the frame watchdog.
        DrmEvent::Error(err) => tracing::error!("display error: {}", err),
    })?;

    // Input devices on this seat.
    let mut libinput = Libinput::new_with_udev(LibinputSessionInterface::from(session.clone()));
    libinput
        .udev_assign_seat(&seat_name)
        .map_err(|_| "libinput could not use the seat")?;
    let libinput_backend = LibinputInputBackend::new(libinput.clone());
    handle.insert_source(libinput_backend, |event, _, state| {
        state.process_input_event(event);
    })?;

    // Session switching (for example to another virtual terminal) pauses input and display.
    handle.insert_source(session_notifier, move |event, _, state| match event {
        SessionEvent::PauseSession => {
            libinput.suspend();
            if let Some(backend) = state.backend.as_mut() {
                backend.drm.pause();
            }
            tracing::info!("session paused");
        }
        SessionEvent::ActivateSession => {
            if let Err(err) = libinput.resume() {
                tracing::warn!("libinput resume: {:?}", err);
            }
            if let Some(backend) = state.backend.as_mut() {
                if let Err(err) = backend.drm.activate(false) {
                    tracing::warn!("display activate: {}", err);
                }
                backend.frame_pending = false;
                backend.compositor.reset_state().ok();
            }
            tracing::info!("session resumed");
            state.render();
        }
    })?;

    state.backend = Some(UdevData {
        session,
        drm,
        renderer,
        compositor,
        output,
        pointer_id: Id::new(),
        frame_pending: false,
        frame_seq: 0,
        redraw_scheduled: false,
        frame_flags,
        render_wait,
        handle: handle.clone(),
    });
    state.pointer_location = state
        .output_geometry()
        .map(|g| (g.loc + g.size.downscale(2).to_point()).to_f64())
        .unwrap_or_default();
    state.render();
    Ok(())
}

/// First connected connector with a CRTC that can drive it, and its preferred mode.
fn find_display(
    fd: &DrmDeviceFd,
) -> Option<(connector::Info, crtc::Handle, smithay::reexports::drm::control::Mode)> {
    let resources = fd.resource_handles().ok()?;
    for handle in resources.connectors() {
        let Ok(info) = fd.get_connector(*handle, true) else { continue };
        if info.state() != connector::State::Connected || info.modes().is_empty() {
            continue;
        }
        let mode = info
            .modes()
            .iter()
            .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
            .copied()
            .unwrap_or(info.modes()[0]);
        for encoder in info.encoders() {
            let Ok(encoder) = fd.get_encoder(*encoder) else { continue };
            if let Some(crtc) = resources.filter_crtcs(encoder.possible_crtcs()).first() {
                return Some((info.clone(), *crtc, mode));
            }
        }
    }
    None
}

impl UnitState {
    /// Draws a frame if none is waiting for its vertical blank. When nothing has changed, checks
    /// again shortly, so new client content is drawn without waiting for input.
    pub fn render(&mut self) {
        let Self {
            backend,
            space,
            pointer_location,
            pointer_visible,
            start_time,
            ..
        } = self;
        let Some(backend) = backend.as_mut() else { return };
        if backend.frame_pending || !backend.session.is_active() {
            return;
        }

        let mut elements: Vec<UnitRenderElements> = Vec::new();
        if *pointer_visible {
            let scale = backend.output.current_scale().fractional_scale();
            let loc = pointer_location.to_physical(scale).to_i32_round::<i32>();
            let size = (POINTER_SIZE as f64 * scale).round() as i32;
            elements.push(UnitRenderElements::Pointer(SolidColorRenderElement::new(
                backend.pointer_id.clone(),
                Rectangle::<i32, Physical>::new(loc, (size, size).into()),
                CommitCounter::default(),
                Color32F::from(POINTER_COLOUR),
                Kind::Cursor,
            )));
        }
        match space.render_elements_for_output(&mut backend.renderer, &backend.output, 1.0) {
            Ok(space_elements) => {
                elements.extend(space_elements.into_iter().map(UnitRenderElements::Space))
            }
            Err(err) => tracing::warn!("space elements: {:?}", err),
        }

        let mut skip_frame = false;
        let rendered = match backend.compositor.render_frame(
            &mut backend.renderer,
            &elements,
            Color32F::from(BACKGROUND),
            backend.frame_flags,
        ) {
            Ok(result) if result.is_empty => false,
            Ok(result) => {
                // Smithay's submission contract: when the display path cannot synchronise with
                // rendering (needs_sync), wait for the frame to finish drawing before queueing it,
                // or the display can show a partly drawn frame (UNIT-OS-ISS-0020). If the wait
                // fails, do not submit the frame; the next frame is drawn in full.
                if result.needs_sync() && backend.render_wait {
                    match &result.primary_element {
                        PrimaryPlaneElement::Swapchain(element) => match element.sync.wait() {
                            Ok(()) => true,
                            Err(err) => {
                                tracing::warn!("frame not shown: waiting for drawing failed: {:?}", err);
                                skip_frame = true;
                                false
                            }
                        },
                        PrimaryPlaneElement::Element(_) => true,
                    }
                } else {
                    true
                }
            }
            Err(err) => {
                tracing::warn!("render: {:?}", err);
                false
            }
        };

        if skip_frame {
            // The skipped frame's damage is lost: draw the next frame in full.
            backend.compositor.reset_buffers();
        }
        if rendered {
            match backend.compositor.queue_frame(()) {
                Ok(()) => {
                    backend.frame_pending = true;
                    backend.frame_seq += 1;
                    let seq = backend.frame_seq;
                    let _ = backend
                        .handle
                        .insert_source(Timer::from_duration(FRAME_TIMEOUT), move |_, _, state| {
                            state.frame_timed_out(seq);
                            TimeoutAction::Drop
                        });
                }
                Err(err) => tracing::warn!("queue frame: {:?}", err),
            }
        }

        // Frame callbacks: clients draw their next frame.
        let output = backend.output.clone();
        let elapsed = start_time.elapsed();
        space.elements().for_each(|window| {
            window.send_frame(&output, elapsed, Some(Duration::ZERO), |_, _: &SurfaceData| {
                Some(output.clone())
            });
        });

        if !backend.frame_pending && !backend.redraw_scheduled {
            backend.redraw_scheduled = true;
            let _ = backend
                .handle
                .insert_source(Timer::from_duration(IDLE_REDRAW), |_, _, state| {
                    if let Some(backend) = state.backend.as_mut() {
                        backend.redraw_scheduled = false;
                    }
                    state.render();
                    TimeoutAction::Drop
                });
        }
    }

    /// Frame watchdog: if frame `seq` is still waiting for its vertical blank, the vertical blank
    /// was lost (for example with a display error). Without this, Smithay keeps the frame pending,
    /// every later frame waits behind it and the display stays frozen. Treats the frame as shown,
    /// re-reads the display state so the next frame is a full commit, and redraws everything.
    /// While the session is paused nothing is done; resuming the session resets the display.
    /// If the lost vertical blank does arrive later, it is counted for the next frame: at worst
    /// that one frame is taken as shown one refresh early.
    fn frame_timed_out(&mut self, seq: u64) {
        let Some(backend) = self.backend.as_mut() else { return };
        if !backend.frame_pending || backend.frame_seq != seq || !backend.session.is_active() {
            return;
        }
        tracing::warn!(
            "frame {} not confirmed by the display after {} ms: redrawing in full",
            seq,
            FRAME_TIMEOUT.as_millis()
        );
        if let Err(err) = backend.compositor.frame_submitted() {
            tracing::warn!("frame submission: {:?}", err);
        }
        if let Err(err) = backend.compositor.reset_state() {
            tracing::warn!("display state reset: {}", err);
        }
        backend.compositor.reset_buffers();
        backend.frame_pending = false;
        self.render();
    }
}
