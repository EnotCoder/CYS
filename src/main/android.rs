// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  android.rs — точка входа под Android (компилируется только для
//  target_os="android"). Использует android-activity + winit.
//  Все игровые файлы (tex/, map.txt, scripts/, sounds/, font.otf) упакованы
//  в APK-assets и читаются через crate::core::asset.
// ========================================================================

use android_activity::AndroidApp;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;

use crate::input::platform::TouchInput;
use crate::App;

#[no_mangle]
fn android_main(app: AndroidApp) {
    // Полный backtrace в logcat (сообщения паники идут в stderr → RustStdoutStderr).
    std::env::set_var("RUST_BACKTRACE", "1");

    // Сохраняем ссылку на AndroidApp для чтения ассетов из APK.
    crate::core::asset::set_android_app(app.clone());

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
    use jni::objects::JObject;
    use jni::objects::JValue;
    use jni::refs::Global;
    use jni::JavaVM;

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let res = vm.attach_current_thread(|env| -> JniResult<()> {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw) }?;
        let window = env.call_method(activity, jni::jni_str!("getWindow"), jni::jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
        let decor = env.call_method(&window, jni::jni_str!("getDecorView"), jni::jni_sig!("()Landroid/view/View;"), &[])?.l()?;
        let flags: i32 = (0x00000002 | 0x00000004 | 0x00000200 | 0x00000100 | 0x00001000) as i32;
        env.call_method(&decor, jni::jni_str!("setSystemUiVisibility"), jni::jni_sig!("(I)V"), &[JValue::Int(flags)])?;
        Ok(())
    });
    if let Err(e) = res {
        eprintln!("hide_system_ui: JNI call failed: {e:?}");
    }
}
