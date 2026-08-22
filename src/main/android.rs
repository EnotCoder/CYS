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
    // Сохраняем ссылку на AndroidApp для чтения ассетов из APK.
    crate::core::asset::set_android_app(app.clone());

    // Цикл событий winit, привязанный к AndroidApp.
    let mut event_loop = EventLoop::builder();
    event_loop.with_android_app(app);
    let event_loop = event_loop.build().expect("failed to build Android event loop");

    // Мобильный ввод — тач-эмуляция мыши (тап/перетаскивание/щипок).
    let mut app_state = App::new(Box::new(TouchInput::new()));

    // Запуск главного цикла (блокирует до выхода/уничтожения активити).
    let _ = event_loop.run_app(&mut app_state);
}
