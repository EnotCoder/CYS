use winit::{
    event::{Event,WindowEvent},
    event_loop::{ControlFlow,EventLoop},
    window::WindowBuilder,
};
use wgpu;


fn main() {
    
    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("game")
        .build(&event_loop)
        .unwrap();
    
    //instance & surface
    let instance = wgpu::Instance::new(
        wgpu::InstanceDescriptor::new_without_display_handle());
    
    let surface = unsafe { instance.create_surface(&window) }
        .expect("Failed to create surface");

    //addapter
    let addapter_option = wgpu::RequestAdapterOptions {
        compatible_surface : Some(&surface),
        ..Default::default()
    };

    let addapter_future = instance.request_adapter(&addapter_option);

    let addapter = pollster::block_on(addapter_future).unwrap();

    println!("{}",addapter.get_info().name);
    

    let window_id = window.id();
    event_loop.run(move | event, event_loop_target | {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            }if window_id == window_id => {
                event_loop_target.exit();
            }
            _ => (),
        }
    }).unwrap();
}
