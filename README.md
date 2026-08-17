<h1 align="center">Create your Shop (CYS)</h1>

<p align="center">
  <img src="logo.png" width="400" alt="Logo">
</p>

<p align="center">
  <img src="Screenshots/s_1.png" width="400" alt="screenshot 1">
  <img src="Screenshots/s_2.png" width="400" alt="screenshot 2">
</p>

<p>
  <strong>✨ Game where you have to set up your own shop ✨</strong>
  <br>
  A 2D sandbox game written in Rust on top of <a href="https://wgpu.rs/">wgpu</a>,
  using <a href="https://docs.rs/winit">winit</a> for windows/input and
  <a href="https://docs.rs/specs">specs</a> for ECS. Currently in Alpha.
</p>

<h2>Features</h2>

<ul>
  <li>Gardening: build your shop — place flooring, decor, walls, outdoor items</li>
  <li>Management: racks with food, cash registers, candy showcases, arcade machines</li>
  <li>Shopper NPCs: spawn, A* pathfinding, queueing at cash registers, making purchases</li>
  <li>Day/night cycle that darkens the scene</li>
  <li>Mini-economy: object prices, rent, bankrupt condition, shop can be opened/closed</li>
  <li>Basement level with two-way transitions</li>
  <li>Settings (VSync toggle, zoom speed), save/load to <code>save.json</code></li>
  <li>Balance and buyer logic are configurable in Lua (<code>scripts/config.lua</code>, <code>scripts/npc.lua</code>)</li>
</ul>

<h2>Status</h2>

<p>
  The game is in Alpha: gameplay and controls may change between builds.
</p>

<h2>Build &amp; Run</h2>

<pre><code>cargo build --release
cargo run --release</code></pre>

<p>Rust edition 2021; uses wgpu 30, winit 0.30, specs 0.18, rodio 0.22, mlua 0.10.</p>

<h2>Controls</h2>

<table>
  <tr><th>Action</th><th>Input</th></tr>
  <tr><td>Action in world (place/remove/interact)</td><td><kbd>LMB</kbd> or <kbd>F</kbd></td></tr>
  <tr><td>Cycle mode (interact / build / delete)</td><td><kbd>Tab</kbd></td></tr>
  <tr><td>Open / close inventory</td><td><kbd>E</kbd></td></tr>
  <tr><td>Toggle shop open/closed</td><td>Click the active icon</td></tr>
  <tr><td>Select hotbar slot</td><td>Click the slot</td></tr>
  <tr><td>Zoom</td><td><kbd>Scroll</kbd>, <kbd>K</kbd> / <kbd>L</kbd></td></tr>
  <tr><td>Move camera</td><td>Drag <kbd>MMB</kbd> or <kbd>Arrow keys</kbd></td></tr>
  <tr><td>Settings</td><td><kbd>Esc</kbd></td></tr>
  <tr><td>Save / Load game</td><td><kbd>Ctrl+S</kbd> / <kbd>Ctrl+L</kbd></td></tr>
  <tr><td>Enter shop</td><td>Click the basement stairs</td></tr>
  <tr><td>Back to menu</td><td><kbd>R</kbd> (when bankrupt)</td></tr>
</table>

<h2>Architecture</h2>

<pre>
src/
├── main/        — entry point (main.rs, App, winit event loop)
├── core/        — kernel: wgpu pipeline, textures, render, constants, util
├── scenes/      — Menu, Game scene, scene manager
├── ecs/         — ECS adapter, components, factory, group
├── data/        — Slot/Object, placement logic, map loading & pathfinding
├── ui/          — HUD, inventory, settings, text renderer
├── input/       — Camera, cursor, interaction
├── npc/         — Shopper NPC logic
├── audio/       — Sound (rodio)
├── scripts/     — Lua bridge (config.lua, npc.lua)
├── doc/         — Documentation
└── tests/       — Unit tests
</pre>

<p>
  Detailed documentation lives in <code>src/doc/</code>:
  <a href="src/doc/ARCHITECTURE.md">ARCHITECTURE.md</a>,
  <a href="src/doc/RS.md">RS.md</a> and
  <a href="src/doc/SCRIPTS.md">SCRIPTS.md</a>.
</p>

<h2>License</h2>

<p>
  This project is licensed under the <strong>GNU General Public License v3.0</strong>.
  See <a href="LICENSE">LICENSE</a> for the full text.
</p>

<p>
  You may view, modify, and distribute the source code. Any derivative work
  must also be open-sourced under GPL-3.0, which prevents proprietary
  commercial forks.
</p>