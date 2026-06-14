<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Create your Shop (CYS) - 2D Building Game</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            color: #e0e0e0;
            line-height: 1.6;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }

        /* Header Section */
        .header {
            text-align: center;
            padding: 3rem 0;
            background: rgba(0, 0, 0, 0.3);
            border-radius: 20px;
            margin-bottom: 2rem;
            backdrop-filter: blur(10px);
        }

        .logo-container {
            margin-bottom: 2rem;
        }

        .logo-placeholder {
            display: inline-block;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            padding: 1rem 2rem;
            border-radius: 15px;
            font-size: 2rem;
            font-weight: bold;
            color: white;
            box-shadow: 0 10px 40px rgba(0,0,0,0.3);
        }

        .logo-img {
            max-width: 200px;
            height: auto;
            filter: drop-shadow(0 4px 20px rgba(0,0,0,0.3));
        }

        h1 {
            font-size: 4rem;
            background: linear-gradient(135deg, #fff 0%, #a8c0ff 100%);
            -webkit-background-clip: text;
            background-clip: text;
            color: transparent;
            margin-bottom: 0.5rem;
            letter-spacing: 2px;
        }

        .tagline {
            font-size: 1.5rem;
            color: #f0a500;
            margin: 1rem 0;
            font-weight: 500;
        }

        .pre-alpha {
            display: inline-block;
            background: #ff6b6b;
            color: white;
            padding: 0.3rem 1rem;
            border-radius: 20px;
            font-size: 0.8rem;
            font-weight: bold;
            margin-top: 1rem;
        }

        /* Description Cards */
        .description {
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 1.5rem;
            margin-bottom: 2rem;
            text-align: center;
            font-size: 1.1rem;
            border: 1px solid rgba(255,255,255,0.2);
        }

        .features-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 1.5rem;
            margin: 2rem 0;
        }

        .feature-card {
            background: rgba(0, 0, 0, 0.4);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 1.5rem;
            transition: transform 0.3s ease, box-shadow 0.3s ease;
            border: 1px solid rgba(255,255,255,0.1);
        }

        .feature-card:hover {
            transform: translateY(-5px);
            box-shadow: 0 10px 30px rgba(0,0,0,0.3);
            border-color: rgba(255,255,255,0.3);
        }

        .feature-icon {
            font-size: 2.5rem;
            margin-bottom: 1rem;
        }

        .feature-card h3 {
            color: #f0a500;
            margin-bottom: 0.8rem;
        }

        /* Controls Section */
        .controls-section {
            background: rgba(0, 0, 0, 0.3);
            border-radius: 15px;
            padding: 1.5rem;
            margin: 2rem 0;
        }

        .controls-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 1rem;
            margin-top: 1rem;
        }

        .control-item {
            display: flex;
            justify-content: space-between;
            padding: 0.5rem;
            border-bottom: 1px solid rgba(255,255,255,0.1);
        }

        .control-key {
            background: #2c2c3e;
            padding: 0.2rem 0.8rem;
            border-radius: 8px;
            font-family: monospace;
            font-weight: bold;
            color: #f0a500;
        }

        .control-action {
            color: #ccc;
        }

        /* Object Table */
        .object-table {
            width: 100%;
            border-collapse: collapse;
            margin: 1rem 0;
        }

        .object-table th,
        .object-table td {
            padding: 0.75rem;
            text-align: left;
            border-bottom: 1px solid rgba(255,255,255,0.1);
        }

        .object-table th {
            color: #f0a500;
            border-bottom: 2px solid #f0a500;
        }

        /* Code Block */
        pre {
            background: #1e1e2e;
            padding: 1rem;
            border-radius: 10px;
            overflow-x: auto;
            font-size: 0.9rem;
            margin: 1rem 0;
        }

        code {
            font-family: 'Courier New', monospace;
            color: #a8c0ff;
        }

        /* Buttons */
        .btn {
            display: inline-block;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 0.8rem 1.5rem;
            border-radius: 8px;
            text-decoration: none;
            font-weight: bold;
            transition: transform 0.2s ease, box-shadow 0.2s ease;
            margin: 0.5rem;
        }

        .btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 5px 20px rgba(102, 126, 234, 0.4);
        }

        .btn-secondary {
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
        }

        /* Footer */
        .footer {
            text-align: center;
            padding: 2rem;
            margin-top: 3rem;
            border-top: 1px solid rgba(255,255,255,0.1);
            color: #888;
        }

        /* Badge */
        .badge {
            display: inline-block;
            background: rgba(255,255,255,0.2);
            padding: 0.2rem 0.6rem;
            border-radius: 10px;
            font-size: 0.8rem;
            margin: 0.2rem;
        }

        hr {
            border: none;
            height: 1px;
            background: linear-gradient(90deg, transparent, #f0a500, transparent);
            margin: 2rem 0;
        }

        @media (max-width: 768px) {
            .container {
                padding: 1rem;
            }
            h1 {
                font-size: 2.5rem;
            }
            .tagline {
                font-size: 1.2rem;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <!-- Logo & Header -->
        <div class="header">
            <div class="logo-container">
                <img src="logo.png" alt="Create your Shop Logo" class="logo-img" onerror="this.style.display='none'">
            </div>
            <h1>✨ Create your Shop (CYS) ✨</h1>
            <div class="tagline">🏪 Game where you have to set up your own shop 🏪</div>
            <div class="pre-alpha">⚡ PRE-ALPHA STAGE ⚡</div>
        </div>

        <!-- Introduction -->
        <div class="description">
            <p>Hello everyone! This is my first 2D game that I made using the <strong>wgpu</strong> API.</p>
            <p>For now, it is my favorite project. I use <strong>winit</strong> for window management and <strong>wgpu</strong> for rendering graphics.</p>
            <p>The game is in <strong>pre-Alpha</strong> stage, and I'm actively developing it!</p>
        </div>

        <!-- Features -->
        <h2>🎮 Game Features</h2>
        <div class="features-grid">
            <div class="feature-card">
                <div class="feature-icon">🏗️</div>
                <h3>Grid-Based Building</h3>
                <p>Place and remove objects on a 9x9 grid system with precise positioning</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🔄</div>
                <h3>Multiple Modes</h3>
                <p>Switch between Standard, Build, and Delete modes for different actions</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">📦</div>
                <h3>6 Inventory Slots</h3>
                <p>Different objects with varying sizes (1x1, 1x2, 2x1) to decorate your shop</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🧩</div>
                <h3>Carpet System</h3>
                <p>Decor objects can only be placed on carpets - strategic placement required!</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🖱️</div>
                <h3>Mouse & Keyboard</h3>
                <p>Scroll to zoom, WASD to move cursor, and intuitive key bindings</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🎨</div>
                <h3>Sprite Atlas Support</h3>
                <p>Objects can use multi-frame textures from texture atlases</p>
            </div>
        </div>

        <hr>

        <!-- Controls -->
        <div class="controls-section">
            <h2>🎮 Controls</h2>
            <div class="controls-grid">
                <div class="control-item">
                    <span class="control-key">W / A / S / D</span>
                    <span class="control-action">Move cursor (200ms move delay)</span>
                </div>
                <div class="control-item">
                    <span class="control-key">F</span>
                    <span class="control-action">Build/Delete (depends on mode)</span>
                </div>
                <div class="control-item">
                    <span class="control-key">Tab</span>
                    <span class="control-action">Cycle modes (Standard → Build → Delete)</span>
                </div>
                <div class="control-item">
                    <span class="control-key">Q</span>
                    <span class="control-action">Switch inventory slot</span>
                </div>
                <div class="control-item">
                    <span class="control-key">K / L</span>
                    <span class="control-action">Zoom in / out</span>
                </div>
                <div class="control-item">
                    <span class="control-key">Mouse Scroll</span>
                    <span class="control-action">Zoom in / out</span>
                </div>
            </div>
        </div>

        <!-- Modes -->
        <h2>🔄 Game Modes</h2>
        <div class="features-grid">
            <div class="feature-card">
                <div class="feature-icon">⭐</div>
                <h3>Standard Mode (0)</h3>
                <p>Default cursor - no building action. Navigate and plan your shop layout.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🟢</div>
                <h3>Build Mode (1)</h3>
                <p>Green/Red cursor - place objects from active slot. Red means invalid position.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🗑️</div>
                <h3>Delete Mode (2)</h3>
                <p>Delete cursor - remove objects under cursor from your shop.</p>
            </div>
        </div>

        <!-- Objects -->
        <h2>📦 Available Objects</h2>
        <table class="object-table">
            <thead>
                <tr>
                    <th>Slot</th>
                    <th>Object</th>
                    <th>Size</th>
                    <th>Type</th>
                </tr>
            </thead>
            <tbody>
                <tr><td>0</td><td>📦 Box</td><td>1x1</td><td>Decor</td></tr>
                <tr><td>1</td><td>🟫 Carpet</td><td>1x1</td><td>Carpet</td></tr>
                <tr><td>2</td><td>📝 Sign</td><td>1x1</td><td>Decor</td></tr>
                <tr><td>3</td><td>🗄️ Rack</td><td>1x2</td><td>Decor</td></tr>
                <tr><td>4</td><td>🪵 Table</td><td>2x1</td><td>Decor</td></tr>
                <tr><td>5</td><td>🔴 Red Carpet</td><td>1x1</td><td>Carpet</td></tr>
            </tbody>
        </table>

        <hr>

        <!-- Technical Stack -->
        <h2>🛠️ Tech Stack</h2>
        <div style="display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 1rem 0;">
            <span class="badge">Rust</span>
            <span class="badge">wgpu 0.19</span>
            <span class="badge">winit 0.29</span>
            <span class="badge">specs 0.18 (ECS)</span>
            <span class="badge">tokio 1.0</span>
            <span class="badge">image 0.24</span>
            <span class="badge">bytemuck 1.16</span>
        </div>

        <!-- Project Structure -->
        <h2>📁 Project Structure</h2>
        <pre>
src/
├── main.rs           # Entry point, window/event loop
├── input.rs          # Input handling & game logic
├── slot_object.rs    # Building objects definition
├── ecs/
│   ├── mod.rs
│   ├── components.rs # ECS components (Transform, Sprite, Group)
│   └── ecs_integration.rs # ECS adapter and game logic
├── sprite_manager.rs # Sprite resource management
└── wgpu/            # Rendering module
    ├── mod.rs
    ├── buffers.rs    # Vertex/uniform buffers
    ├── init.rs       # WGPU initialization
    ├── render.rs     # Rendering pipeline
    └── texture.rs    # Texture loading
        </pre>

        <!-- Building -->
        <h2>🔧 Building & Running</h2>
        <div class="controls-section">
            <h3>Prerequisites</h3>
            <p>• Rust (latest stable)<br>• Vulkan/Metal/DirectX 12 compatible GPU</p>
            
            <h3>Build</h3>
            <pre><code>cargo build --release</code></pre>
            
            <h3>Run</h3>
            <pre><code>cargo run</code></pre>
            
            <h3>Map Format</h3>
            <p>The game loads <code>map.txt</code> with tile codes for grass, floor, and wall variants.</p>
        </div>

        <!-- Assets -->
        <h2>🖼️ Asset Structure</h2>
        <pre>
tex/
├── grass.png, floor.png, wall.png  # Map tiles
├── decor/
│   ├── box.png, carpet.png, sign.png
│   ├── rack.png, table.png
├── cursor/
│   ├── def_cursor.png, cursor.png
│   ├── del_cursor.png, err cursor.png
├── ui/
│   ├── standart_mode.png, build_mode.png, del_mode.png
│   └── icon_slots/
└── null.png  # Fallback texture
        </pre>

        <hr>

        <!-- Known Limitations -->
        <h2>⚠️ Known Limitations</h2>
        <div class="description">
            <p>• Grid is fixed at 9x9 cells (-4 to 5)<br>
            • Objects cannot be placed outside grid boundaries<br>
            • Only one object per cell<br>
            • Decor objects require a carpet underneath</p>
        </div>

        <!-- Footer -->
        <div class="footer">
            <p>Made with ❤️ using Rust and wgpu</p>
            <p>This is an open-source project - contributions and suggestions are welcome!</p>
            <p>📧 <strong>Note:</strong> This README is best viewed on GitHub or with a markdown previewer that supports HTML</p>
        </div>
    </div>
</body>
</html>
