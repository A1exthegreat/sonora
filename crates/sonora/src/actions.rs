use gpui::{App, Context, Menu, MenuItem, OsAction};
use i18n::t;
use input::{
    ClearRating, CloseWindow, DecreaseFontSize, DiscardTrack, Hide, HideOthers, IncreaseFontSize,
    MinimizeWindow, OpenSettings, Quit, RateFive, RateFour, RateOne, RateThree, RateTwo, Redo,
    Refresh, ResetFontSize, ShowAll, SignOut, SongNext, SongPrevious, TogglePlayback,
    ToggleRepeat, ToggleShuffle, ToggleTrackSaved, Undo, ZoomWindow,
};
use router::Destination;
use state::{decisions, AppSettings, Equalizer, Io, Outcome, Shelf, Sonora, Toast, Toasts};
use ui::{ActiveTheme as _, Copy, Cut, Look, Paste, SelectAll, Theme};

/// How far one font size shortcut moves the size, the same step as the buttons in Settings.
const FONT_STEP: f32 = 1.;

pub fn register(lingers: bool, cx: &mut App) {
    cx.bind_keys(input::bindings());

    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &Hide, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());

    cx.on_window_closed(move |cx, _| {
        if !cx.windows().is_empty() {
            return;
        }
        let close_to_tray = Sonora::global(cx).settings.read(cx).close_to_tray();
        match lingers && close_to_tray {
            true => crate::dock::show(false),
            false => cx.quit(),
        }
    })
    .detach();

    cx.on_action(|_: &SignOut, cx: &mut App| {
        let session = Sonora::global(cx).session.clone();
        session.update(cx, |session, cx| session.sign_out(cx));
    });

    // Root handles Refresh whenever a window is open; this runs only without one (the macOS menu).
    cx.on_action(
        |_: &Refresh, cx: &mut App| match router::trail(cx).read(cx).current() {
            Destination::History => {
                let history = Sonora::global(cx).history.clone();
                history.update(cx, |history, cx| history.refresh(cx));
            }
            at if !at.refreshes() => {}
            at => {
                let shelf = match at {
                    Destination::Local(_) => Shelf::Local,
                    Destination::Album(id) | Destination::Playlist(id) => Shelf::of(&id),
                    _ => Shelf::Streaming,
                };
                let library = Sonora::global(cx).library.clone();
                library.update(cx, |library, cx| library.refresh(shelf, cx));
            }
        },
    );

    cx.on_action(|_: &TogglePlayback, cx: &mut App| {
        let playback = Sonora::global(cx).playback.clone();
        playback.update(cx, |playback, cx| playback.toggle_play(cx));
    });

    cx.on_action(|_: &SongPrevious, cx: &mut App| {
        let playback = Sonora::global(cx).playback.clone();
        playback.update(cx, |playback, cx| playback.previous(cx));
    });

    cx.on_action(|_: &SongNext, cx: &mut App| {
        let playback = Sonora::global(cx).playback.clone();
        playback.update(cx, |playback, cx| playback.next(cx));
    });

    cx.on_action(|_: &ToggleShuffle, cx: &mut App| {
        let playback = Sonora::global(cx).playback.clone();
        playback.update(cx, |playback, cx| playback.toggle_shuffle(cx));
    });

    cx.on_action(|_: &ToggleRepeat, cx: &mut App| {
        let playback = Sonora::global(cx).playback.clone();
        playback.update(cx, |playback, cx| playback.toggle_repeat(cx));
    });

    cx.on_action(|_: &RateOne, cx: &mut App| rate(cx, 1));
    cx.on_action(|_: &RateTwo, cx: &mut App| rate(cx, 2));
    cx.on_action(|_: &RateThree, cx: &mut App| rate(cx, 3));
    cx.on_action(|_: &RateFour, cx: &mut App| rate(cx, 4));
    cx.on_action(|_: &RateFive, cx: &mut App| rate(cx, 5));
    cx.on_action(|_: &ClearRating, cx: &mut App| rate(cx, 0));
    cx.on_action(|_: &ToggleTrackSaved, cx: &mut App| toggle_saved(cx));
    cx.on_action(|_: &DiscardTrack, cx: &mut App| discard(cx));

    cx.on_action(|_: &Undo, cx: &mut App| edit_equalizer(cx, Equalizer::undo));
    cx.on_action(|_: &Redo, cx: &mut App| edit_equalizer(cx, Equalizer::redo));

    cx.on_action(|_: &IncreaseFontSize, cx: &mut App| {
        change_font_size(cx, |settings, cx| {
            settings.set_font_size(settings.font_size() + FONT_STEP, cx)
        });
    });

    cx.on_action(|_: &DecreaseFontSize, cx: &mut App| {
        change_font_size(cx, |settings, cx| {
            settings.set_font_size(settings.font_size() - FONT_STEP, cx)
        });
    });

    cx.on_action(|_: &ResetFontSize, cx: &mut App| {
        change_font_size(cx, |settings, cx| settings.reset_font_size(cx));
    });

    cx.set_menus(menus());
}

/// The menu bar. Only macOS draws one, so only macOS gets the Edit and Window menus and the
/// application-menu items Cocoa users expect; the other platforms keep the one Sonora menu.
fn menus() -> Vec<Menu> {
    let app = match cfg!(target_os = "macos") {
        true => vec![
            MenuItem::action(t!("app-settings"), OpenSettings),
            MenuItem::separator(),
            MenuItem::action(t!("app-refresh"), Refresh),
            MenuItem::action(t!("app-sign-out"), SignOut),
            MenuItem::separator(),
            MenuItem::action(t!("app-hide"), Hide),
            MenuItem::action(t!("app-hide-others"), HideOthers),
            MenuItem::action(t!("app-show-all"), ShowAll),
            MenuItem::separator(),
            MenuItem::action(t!("app-quit"), Quit),
        ],
        false => vec![
            MenuItem::action(t!("app-refresh"), Refresh),
            MenuItem::action(t!("app-sign-out"), SignOut),
            MenuItem::separator(),
            MenuItem::action(t!("app-quit"), Quit),
        ],
    };
    let mut menus = vec![Menu {
        name: "Sonora".into(),
        disabled: false,
        items: app,
    }];
    if cfg!(target_os = "macos") {
        menus.push(Menu {
            name: t!("app-edit"),
            disabled: false,
            items: vec![
                MenuItem::os_action(t!("app-cut"), Cut, OsAction::Cut),
                MenuItem::os_action(t!("app-copy"), Copy, OsAction::Copy),
                MenuItem::os_action(t!("app-paste"), Paste, OsAction::Paste),
                MenuItem::os_action(t!("app-select-all"), SelectAll, OsAction::SelectAll),
            ],
        });
        menus.push(Menu {
            name: t!("app-window"),
            disabled: false,
            items: vec![
                MenuItem::action(t!("app-close-window"), CloseWindow),
                MenuItem::action(t!("app-minimize"), MinimizeWindow),
                MenuItem::action(t!("app-zoom"), ZoomWindow),
            ],
        });
    }
    menus
}

/// Runs `edit` on the equalizer while its screen is the one shown, the only place undo applies.
fn edit_equalizer(cx: &mut App, edit: fn(&mut Equalizer, &mut Context<Equalizer>)) {
    if router::trail(cx).read(cx).current() == Destination::Equalizer {
        let equalizer = Sonora::global(cx).equalizer.clone();
        equalizer.update(cx, edit);
    }
}

/// The track a rating shortcut acts on: the one the player shows, which is the other device's
/// while steering, and the provider whose id it is.
fn shown(cx: &App) -> Option<(music::Track, Option<String>)> {
    let sonora = Sonora::global(cx);
    let track = sonora.playback.read(cx).shown_track(cx)?;
    let id = track.id.as_deref()?;
    let provider = match music::is_local_id(id) {
        true => Some("local".to_owned()),
        false => sonora.session.read(cx).provider_slug().map(str::to_owned),
    };
    Some((track, provider))
}

/// Rates the track shown, records the decision and tells the provider that keeps ratings.
fn rate(cx: &mut App, stars: u8) {
    let Some((track, provider)) = shown(cx) else {
        return;
    };
    let id = track.id.clone().expect("shown returns a track with an id");
    let client = match music::is_local_id(&id) {
        true => None,
        false => Sonora::global(cx).session.read(cx).client(),
    };
    decisions::record(decisions::Choice::Rate(stars), &track, provider);
    match client {
        None => {
            Toasts::post(rated(stars, track.name.clone()), cx);
        }
        Some(client) => {
            let io = Io::global(cx);
            let name = track.name.clone();
            cx.spawn(async move |cx| {
                let failed = match io
                    .spawn(async move { client.set_track_rating(&id, stars).await })
                    .await
                {
                    Ok(Ok(())) => false,
                    Ok(Err(error)) => {
                        log::warn!("rating: cannot set {stars} stars: {error:#}");
                        true
                    }
                    Err(error) => {
                        log::warn!("rating: the rating task failed: {error}");
                        true
                    }
                };
                cx.update(|cx| match failed {
                    true => Toasts::post(
                        Toast::new(Outcome::Failed, "toast-rating-failed").name(name),
                        cx,
                    ),
                    false => Toasts::post(rated(stars, name), cx),
                });
            })
            .detach();
        }
    }
}

/// The toast that says a rating landed, with `stars` out of five, or that it was cleared.
fn rated(stars: u8, name: String) -> Toast {
    let mut toast = Toast::new(
        Outcome::Done,
        match stars {
            0 => "toast-rating-cleared",
            _ => "toast-rated",
        },
    )
    .name(name);
    if stars > 0 {
        toast = toast.count(usize::from(stars));
    }
    toast
}

/// Saves the track shown to the library, or takes it out, and records the decision.
fn toggle_saved(cx: &mut App) {
    let Some((track, provider)) = shown(cx) else {
        return;
    };
    let id = track.id.clone().expect("shown returns a track with an id");
    let sonora = Sonora::global(cx);
    let saved = sonora.library.read(cx).saved(&id);
    let library = sonora.library.clone();
    decisions::record(decisions::Choice::Favorite(!saved), &track, provider);
    library.update(cx, |library, cx| library.toggle(track, cx));
}

/// Records that the track shown is to be left out, then plays the next one.
fn discard(cx: &mut App) {
    let Some((track, provider)) = shown(cx) else {
        return;
    };
    decisions::record(decisions::Choice::Discard, &track, provider);
    Toasts::post(
        Toast::new(Outcome::Done, "toast-discarded").name(track.name.clone()),
        cx,
    );
    let playback = Sonora::global(cx).playback.clone();
    playback.update(cx, |playback, cx| playback.next(cx));
}

/// Applies `change` to the saved font size and redraws with it, keeping the cover tint the
/// theme already wears so a shortcut does not undo the adaptive palette.
fn change_font_size(
    cx: &mut App,
    change: impl FnOnce(&mut AppSettings, &mut Context<AppSettings>),
) {
    let settings = Sonora::global(cx).settings.clone();
    settings.update(cx, change);
    let (tint, tint_secondary) = (cx.theme().tint, cx.theme().tint_secondary);
    let (look, overrides) = {
        let settings = settings.read(cx);
        (
            Look {
                tint,
                tint_secondary,
                ..settings.look()
            },
            settings.theme_overrides(),
        )
    };
    Theme::set(look, &overrides, cx);
}
