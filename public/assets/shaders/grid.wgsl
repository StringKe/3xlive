// 网格背景 shader：黑色背景 + 蓝色网格线 + 穿梭光效

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct GridUniforms {
    camera_pos: vec2<f32>,
    view_size: vec2<f32>,
    time: f32,
    tile_size: f32,
    _padding: vec2<f32>,
}

@group(2) @binding(0) var<uniform> grid: GridUniforms;

// 伪随机 hash
fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let n = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(n) * 43758.5453);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // UV → 世界坐标（Bevy UV y=0 顶部，翻转 V 轴）
    let world_pos = grid.camera_pos + vec2<f32>(in.uv.x - 0.5, 0.5 - in.uv.y) * grid.view_size;

    let tile = grid.tile_size;

    // ─── 网格线 ───
    let grid_frac = abs(fract(world_pos / tile + 0.5) - 0.5) * tile;
    let grid_dist = min(grid_frac.x, grid_frac.y);
    // 线宽根据世界→屏幕缩放自适应（fwidth 返回相邻像素的 world_pos 变化量）
    let pixel_width = length(vec2<f32>(dpdx(world_pos.x), dpdy(world_pos.y)));
    let line_width = max(pixel_width * 1.2, 0.5);
    let line = 1.0 - smoothstep(0.0, line_width, grid_dist);

    let blue = vec3<f32>(0.1, 0.45, 0.9);
    let line_color = blue * line * 0.45;

    // ─── 穿梭光效 ───
    let cyan = vec3<f32>(0.0, 0.894, 1.0);
    var streak_acc = 0.0;
    let streak_range = max(grid.view_size.x, grid.view_size.y) * 1.5;

    // 水平穿梭（沿 y = N * tile）
    let h_idx = round(world_pos.y / tile);
    let h_y = h_idx * tile;
    let h_dy = abs(world_pos.y - h_y);
    if h_dy < 3.0 {
        let across_h = 1.0 - smoothstep(0.0, 1.5, h_dy);
        let h_active = hash21(vec2<f32>(h_idx, 0.0));
        if h_active > 0.9 {
            for (var si = 0; si < 1; si++) {
                let seed = vec2<f32>(h_idx + 50.0, f32(si) + 0.5);
                let rnd = hash22(seed);
                let speed = (250.0 + rnd.x * 400.0) * select(-1.0, 1.0, rnd.y > 0.5);
                let streak_len = 30.0 + rnd.y * 50.0;
                let dir = sign(speed);

                let head_x = grid.camera_pos.x + (fract(rnd.x * 100.0 + grid.time * speed / streak_range) - 0.5) * streak_range;
                let dx = (head_x - world_pos.x) * dir;

                if dx > -1.0 && dx < streak_len {
                    let tail_t = clamp(dx / streak_len, 0.0, 1.0);
                    let brightness = pow(1.0 - tail_t, 2.0);
                    streak_acc += brightness * across_h * 0.25;
                }
            }
        }
    }

    // 竖直穿梭（沿 x = N * tile）
    let v_idx = round(world_pos.x / tile);
    let v_x = v_idx * tile;
    let v_dx = abs(world_pos.x - v_x);
    if v_dx < 3.0 {
        let across_v = 1.0 - smoothstep(0.0, 1.5, v_dx);
        let v_active = hash21(vec2<f32>(v_idx, 100.0));
        if v_active > 0.9 {
            for (var si = 0; si < 1; si++) {
                let seed = vec2<f32>(v_idx + 200.0, f32(si) + 0.5);
                let rnd = hash22(seed);
                let speed = (220.0 + rnd.x * 380.0) * select(-1.0, 1.0, rnd.y > 0.5);
                let streak_len = 25.0 + rnd.y * 45.0;
                let dir = sign(speed);

                let head_y = grid.camera_pos.y + (fract(rnd.x * 100.0 + grid.time * speed / streak_range) - 0.5) * streak_range;
                let dy = (head_y - world_pos.y) * dir;

                if dy > -1.0 && dy < streak_len {
                    let tail_t = clamp(dy / streak_len, 0.0, 1.0);
                    let brightness = pow(1.0 - tail_t, 2.0);
                    streak_acc += brightness * across_v * 0.22;
                }
            }
        }
    }

    let streak_color = cyan * streak_acc;

    // ─── 合成 ───
    let bg = vec3<f32>(0.02, 0.02, 0.04);

    return vec4<f32>(bg + line_color + streak_color, 1.0);
}
