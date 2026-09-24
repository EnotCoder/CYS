// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  android.rs — точка входа под Android (компилируется только для
//  target_os="android"). Использует android-activity + winit.
//  Все игровые файлы (tex/, map.txt, scripts/, sounds/, font.otf) упакованы
//  в APK-assets и читаются через crate::core::asset.
// ========================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use android_activity::AndroidApp;
use jni::objects::{JObject, JString, JValue};
use jni::refs::Global;
use jni::JavaVM;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;

macro_rules! jni_name {
    ($name:literal) => { jni::jni_str!($name) };
}

macro_rules! jni_sig {
    ($signature:literal) => { jni::jni_sig!($signature) };
}

use crate::input::platform::TouchInput;
use crate::App;

#[no_mangle]
fn android_main(app: AndroidApp) {
    // Полный backtrace в logcat (сообщения паники идут в stderr → RustStdoutStderr).
    std::env::set_var("RUST_BACKTRACE", "1");

    // Сохраняем ссылку на AndroidApp для чтения ассетов из APK.
    crate::core::asset::set_android_app(app.clone());
    init_text_input(&app);

    // Прячем системную статус-панель (время/батарея/уведомления) — как в
    // обычных мобильных играх. FLAG_FULLSCREEN скрывает верхнюю панель.
    app.set_window_flags(
        android_activity::WindowManagerFlags::FULLSCREEN,
        android_activity::WindowManagerFlags::empty(),
    );

    // Прячем системную навигационную панель (кнопки back/home/recents) в
    // иммерсивном режиме, как в других мобильных играх. Эти флаги относятся
    // к View.setSystemUiVisibility, а не к WindowManager, поэтому ставим их
    // через JNI (WindowManagerFlags в android-activity их не содержит).
    hide_system_ui(&app);

    // Цикл событий winit, привязанный к AndroidApp.
    let mut event_loop = EventLoop::builder();
    event_loop.with_android_app(app);
    let event_loop = event_loop.build().expect("failed to build Android event loop");

    // Мобильный ввод — тач-эмуляция мыши (тап/перетаскивание/щипок).
    let mut app_state = App::new(Box::new(TouchInput::new()));

    // Запуск главного цикла (блокирует до выхода/уничтожения активити).
    let _ = event_loop.run_app(&mut app_state);
}

// Скрывает статус- и навигационную панели через View.setSystemUiVisibility в
// иммерсивном «липком» режиме (SYSTEM_UI_FLAG_IMMERSIVE_STICKY): панели сами
// возвращаются после свайпа и снова прячутся, контент лежит под ними, поэтому
// winit не меняет размер окна. Флаги (сумма бит):
//   HIDE_NAVIGATION         0x00000002
//   FULLSCREEN              0x00000004
//   LAYOUT_HIDE_NAVIGATION  0x00000200
//   LAYOUT_FULLSCREEN       0x00000100
//   IMMERSIVE_STICKY        0x00001000
fn hide_system_ui(app: &AndroidApp) {
    use jni::errors::Result as JniResult;

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let res = vm.attach_current_thread(|env| -> JniResult<()> {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = std::mem::ManuallyDrop::new(unsafe { JObject::from_raw(env, raw) });
        let window = env.call_method(&*activity, jni::jni_str!("getWindow"), jni::jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
        let decor = env.call_method(&window, jni::jni_str!("getDecorView"), jni::jni_sig!("()Landroid/view/View;"), &[])?.l()?;
        let flags: i32 = (0x00000002 | 0x00000004 | 0x00000200 | 0x00000100 | 0x00001000) as i32;
        env.call_method(&decor, jni::jni_str!("setSystemUiVisibility"), jni::jni_sig!("(I)V"), &[JValue::Int(flags)])?;
        Ok(())
    });
    if let Err(e) = res {
        eprintln!("hide_system_ui: JNI call failed: {e:?}");
    }
}

struct AndroidTextShared {
    edit: Option<Arc<Global<JObject<'static>>>>,
    value: Option<(String, bool)>,
    last_value: Option<String>,
    active: bool,
    ready: bool,
}

struct AndroidTextInput {
    app: AndroidApp,
    shared: Arc<Mutex<AndroidTextShared>>,
    creating: Arc<AtomicBool>,
    poll_pending: Arc<AtomicBool>,
}

static TEXT_INPUT: OnceLock<Mutex<Option<AndroidTextInput>>> = OnceLock::new();

fn text_input_slot() -> &'static Mutex<Option<AndroidTextInput>> {
    TEXT_INPUT.get_or_init(|| Mutex::new(None))
}

pub(crate) fn init_text_input(app: &AndroidApp) {
    *text_input_slot().lock().unwrap() = Some(AndroidTextInput {
        app: app.clone(),
        shared: Arc::new(Mutex::new(AndroidTextShared {
            edit: None,
            value: None,
            last_value: None,
            active: false,
            ready: false,
        })),
        creating: Arc::new(AtomicBool::new(false)),
        poll_pending: Arc::new(AtomicBool::new(false)),
    });
}

pub(crate) fn poll_text_input() {
    let input = text_input_slot().lock().unwrap().as_ref().map(|input| {
        (input.app.clone(), input.shared.clone(), input.poll_pending.clone())
    });
    let Some((app, shared, poll_pending)) = input else { return };
    let edit = {
        let state = shared.lock().unwrap();
        if !state.active { return; }
        state.edit.clone()
    };
    let Some(edit) = edit else { return };
    if poll_pending.swap(true, Ordering::AcqRel) { return; }

    app.clone().run_on_java_main_thread(Box::new(move || {
        let result = read_edit_text(&app, &edit);
        let mut state = shared.lock().unwrap();
        poll_pending.store(false, Ordering::Release);
        if state.active {
            if let Ok(text) = result {
                store_changed_text(&mut state, text);
            }
        }
    }));
}

pub(crate) fn sync_text_input(active: bool, focus_requested: bool) {
    let input = text_input_slot().lock().unwrap().as_ref().map(|input| {
        (input.app.clone(), input.shared.clone(), input.creating.clone(), input.poll_pending.clone())
    });
    let Some((app, shared, creating, poll_pending)) = input else { return };
    let initial = crate::ui::text_input::TEXT_INPUT.focus_text();
    let edit = {
        let mut state = shared.lock().unwrap();
        state.active = active;
        if !active {
            state.value = None;
        } else if focus_requested {
            state.value = None;
        }
        state.edit.clone()
    };

    if !active {
        if let Some(edit) = edit {
            hide_edit_text(&app, edit);
        }
        return;
    }

    if let Some(edit) = edit {
        if focus_requested {
            focus_edit_text(&app, edit.clone(), Some(initial));
        }
        if !poll_pending.swap(true, Ordering::AcqRel) {
            app.clone().run_on_java_main_thread(Box::new(move || {
                let result = read_edit_text(&app, &edit);
                let mut state = shared.lock().unwrap();
                poll_pending.store(false, Ordering::Release);
                if state.active {
                    if let Ok(text) = result {
                        store_changed_text(&mut state, text);
                    }
                }
            }));
        }
    } else if !creating.swap(true, Ordering::AcqRel) {
        let shared_for_create = shared.clone();
        let app_for_create = app.clone();
        app_for_create.clone().run_on_java_main_thread(Box::new(move || {
            let result = create_edit_text(&app_for_create, &initial);
            if result.is_ok() {
                app_for_create.show_soft_input(true);
            }
            creating.store(false, Ordering::Release);
            let mut state = shared_for_create.lock().unwrap();
            match result {
                Ok(edit) => {
                    state.edit = Some(Arc::new(edit));
                    state.ready = true;
                    state.last_value = Some(initial.clone());
                    state.value = None;
                }
                Err(error) => eprintln!("create Android text input: {error:?}"),
            }
        }));
        app.show_soft_input(true);
    }
}

pub(crate) fn text_input_value() -> Option<(String, bool)> {
    text_input_slot()
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|input| {
            let mut state = input.shared.lock().unwrap();
            if state.ready { state.value.take() } else { None }
        })
}

pub(crate) fn set_text_input_value(value: &str) {
    let input = text_input_slot().lock().unwrap().as_ref().map(|input| {
        (input.app.clone(), input.shared.clone())
    });
    let Some((app, shared)) = input else { return };
    let edit = {
        let mut state = shared.lock().unwrap();
        state.last_value = Some(value.to_string());
        state.value = None;
        state.edit.clone()
    };
    if let Some(edit) = edit {
        let value = value.to_string();
        app.clone().run_on_java_main_thread(Box::new(move || {
            if let Err(error) = set_edit_text_value(&app, &edit, &value) {
                eprintln!("set Android text input: {error:?}");
            }
        }));
    }
}

fn normalize_text(text: String) -> (String, bool) {
    let enter = text.contains('\n') || text.contains('\r');
    let text = text.chars().filter(|ch| !matches!(ch, '\n' | '\r')).collect();
    (text, enter)
}

fn store_changed_text(state: &mut AndroidTextShared, text: String) {
    let (text, enter) = normalize_text(text);
    if state.last_value.as_deref() != Some(text.as_str()) {
        state.last_value = Some(text.clone());
        state.value = Some((text, enter));
    }
}

fn create_edit_text(app: &AndroidApp, initial: &str) -> jni::errors::Result<Global<JObject<'static>>> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = std::mem::ManuallyDrop::new(unsafe { JObject::from_raw(env, raw) });
        let edit = env.new_object(jni_name!("android/widget/EditText"), jni_sig!("()V"), &[])?;
        configure_edit_text(env, &edit)?;
        set_edit_text(env, &edit, initial)?;

        let window = env.call_method(&*activity, jni_name!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
        let decor = env.call_method(&window, jni_name!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?.l()?;
        let params = env.new_object(
            jni_name!("android/view/ViewGroup$LayoutParams"),
            jni_sig!("(II)V"),
            &[JValue::Int(1), JValue::Int(1)],
        )?;
        env.call_method(
            &decor,
            jni_name!("addView"),
            jni_sig!("(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"),
            &[JValue::Object(&edit), JValue::Object(&params)],
        )?;
        env.call_method(&edit, jni_name!("requestFocus"), jni_sig!("()Z"), &[])?;
        env.new_global_ref(edit)
    })
}

fn configure_edit_text(env: &mut jni::Env, edit: &JObject) -> jni::errors::Result<()> {
    env.call_method(edit, jni_name!("setSingleLine"), jni_sig!("(Z)V"), &[JValue::Bool(false)])?;
    env.call_method(edit, jni_name!("setInputType"), jni_sig!("(I)V"), &[JValue::Int(0x0002_0001)])?;
    env.call_method(edit, jni_name!("setImeOptions"), jni_sig!("(I)V"), &[JValue::Int(0)])?;
    env.call_method(edit, jni_name!("setBackgroundColor"), jni_sig!("(I)V"), &[JValue::Int(0)])?;
    env.call_method(edit, jni_name!("setTextColor"), jni_sig!("(I)V"), &[JValue::Int(0)])?;
    env.call_method(edit, jni_name!("setCursorVisible"), jni_sig!("(Z)V"), &[JValue::Bool(false)])?;
    env.call_method(edit, jni_name!("setFocusable"), jni_sig!("(Z)V"), &[JValue::Bool(true)])?;
    env.call_method(edit, jni_name!("setFocusableInTouchMode"), jni_sig!("(Z)V"), &[JValue::Bool(true)])?;
    env.call_method(edit, jni_name!("setShowSoftInputOnFocus"), jni_sig!("(Z)V"), &[JValue::Bool(true)])?;
    env.call_method(edit, jni_name!("setAlpha"), jni_sig!("(F)V"), &[JValue::Float(0.0)])?;
    Ok(())
}

fn set_edit_text(env: &mut jni::Env, edit: &JObject, value: &str) -> jni::errors::Result<()> {
    let value = JString::new(env, value)?;
    env.call_method(edit, jni_name!("setText"), jni_sig!("(Ljava/lang/CharSequence;)V"), &[JValue::Object(&value)])?;
    Ok(())
}

fn set_edit_text_value(app: &AndroidApp, edit: &Global<JObject<'static>>, value: &str) -> jni::errors::Result<()> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| set_edit_text(env, edit.as_obj(), value))
}

fn focus_edit_text(app: &AndroidApp, edit: Arc<Global<JObject<'static>>>, initial: Option<String>) {
    let app_for_thread = app.clone();
    let edit_for_thread = edit.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { JavaVM::from_raw(app_for_thread.vm_as_ptr().cast()) };
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            if let Some(value) = &initial {
                set_edit_text(env, edit_for_thread.as_obj(), value)?;
            }
            env.call_method(edit_for_thread.as_obj(), jni_name!("requestFocus"), jni_sig!("()Z"), &[])?;
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("focus Android text input: {error:?}");
        } else {
            app_for_thread.show_soft_input(true);
        }
    }));
    app.show_soft_input(true);
}

fn hide_edit_text(app: &AndroidApp, edit: Arc<Global<JObject<'static>>>) {
    let app_for_thread = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { JavaVM::from_raw(app_for_thread.vm_as_ptr().cast()) };
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            env.call_method(edit.as_obj(), jni_name!("clearFocus"), jni_sig!("()V"), &[])?;
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("hide Android text input: {error:?}");
        }
    }));
    app.hide_soft_input(true);
}

fn read_edit_text(app: &AndroidApp, edit: &Global<JObject<'static>>) -> jni::errors::Result<String> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        let editable = env.call_method(edit.as_obj(), jni_name!("getText"), jni_sig!("()Landroid/text/Editable;"), &[])?.l()?;
        let string = env.call_method(&editable, jni_name!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let string = unsafe { JString::from_raw(env, string.into_raw()) };
        string.try_to_string(env)
    })
}
