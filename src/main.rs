use gstreamer::prelude::*;
use gstreamer::MessageView;
use gstreamer::*;
use ctrlc;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let sdp_path = String::from("/Users/varundeepsaini/RustroverProjects/gstreamer-lib/input-h264.sdp");
    let mp4_temp_path = String::from("/Users/varundeepsaini/RustroverProjects/gstreamer-lib/output.mp4.tmp");
    let mp4_path = String::from("/Users/varundeepsaini/RustroverProjects/gstreamer-lib/output.mp4");

    init()?;

    let pipeline = Pipeline::with_name("gstreamer-pipeline");

    let file_src = ElementFactory::make("filesrc")
        .name("filesrc")
        .property("location", sdp_path)
        .build()
        .expect("Error creating file_src element");

    let sdp_demux = ElementFactory::make("sdpdemux")
        .name("demux")
        .property("timeout", 0u64)
        .build()
        .expect("Error creating sdp_demux element");

    let mp4mux = ElementFactory::make("mp4mux")
        .name("mux")
        .property("faststart", &true)
        .property("faststart-file", mp4_temp_path)
        .build()
        .expect("Error creating mp4mux element");

    let file_sink = ElementFactory::make("filesink")
        .name("filesink")
        .property("location", mp4_path)
        .build()
        .expect("Error creating file_sink element");

    let queue_opus = ElementFactory::make("queue")
        .name("queue_opus")
        .build()
        .expect("Error creating queue_opus element");

    let rtp_opus_depay = ElementFactory::make("rtpopusdepay")
        .name("rtpopusdepay")
        .build()
        .expect("Error creating rtp_opus_depay element");

    let opus_parse = ElementFactory::make("opusparse")
        .name("opusparse")
        .build()
        .expect("Error creating opus_parse element");

    let queue_h264 = ElementFactory::make("queue")
        .name("queue_h264")
        .build()
        .expect("Error creating queue_h264 element");

    let rtp_h264_depay = ElementFactory::make("rtph264depay")
        .name("rtph264depay")
        .build()
        .expect("Error creating rtp_h264_depay element");

    let h264_parse = ElementFactory::make("h264parse")
        .name("h264parse")
        .build()
        .expect("Error creating h264_parse element");

    pipeline.add_many(&[
        &file_src, &sdp_demux, &mp4mux, &file_sink,
        &queue_opus, &rtp_opus_depay, &opus_parse,
        &queue_h264, &rtp_h264_depay, &h264_parse,
    ]).expect("");

    gstreamer::Element::link_many(&[&file_src , &sdp_demux])
        .expect("Error linking file_src and sdp_demux elements");

    gstreamer::Element::link_many(&[&mp4mux, &file_sink])
        .expect("Error linking mp4_mux and file_sink elements");

    let queue_opus_clone = queue_opus.clone();
    let rtp_opus_depay_clone = rtp_opus_depay.clone();
    let opus_parse_clone = opus_parse.clone();
    let mp4mux_opus_clone = mp4mux.clone();

    let queue_h264_clone = queue_h264.clone();
    let rtp_h264_depay_clone = rtp_h264_depay.clone();
    let h264_parse_clone = h264_parse.clone();
    let mp4mux_h264_clone = mp4mux.clone();

    sdp_demux.connect_pad_added(move | _, src_pad| {
        println!("Pad added: {}", src_pad.name());   

        println!("{:#?}", src_pad);

        let actual_pad = if let Some(ghost_pad) = src_pad.downcast_ref::<gstreamer::GhostPad>() {
            if let Some(target_pad) = ghost_pad.target() {
                println!("GhostPad linked to target pad: {}", target_pad.name());
                target_pad
            } else {
                eprintln!("GhostPad has no target pad.");
                return;
            }
        } else {
            src_pad.clone()
        };


    
        let caps = match actual_pad.current_caps() {
            Some(caps) => caps,
            None => {
                eprintln!("Failed to get caps from pad: {}", actual_pad.name());
                return;
            }
        };
    
        let media_type = caps
            .structure(0)
            .and_then(|s| s.get::<&str>("media").ok())
            .unwrap_or("unknown");
    
        println!("New pad added with media type: {}", media_type);
    
        if media_type == "audio" {


            let sink_pad = queue_opus_clone
                .static_pad("sink")
                .expect("Failed to get sink pad from queue_opus_clone.");
            match src_pad.link(&sink_pad) {
                Ok(_) => println!("Linked audio pad to queue_opus_clone."),
                Err(err) => {
                    eprintln!("Failed to link audio pad: {}", err);
                    return;
                }
            }
    
            match gstreamer::Element::link_many(&[
                &queue_opus_clone,
                &rtp_opus_depay_clone,
                &opus_parse_clone,
                &mp4mux_opus_clone,
            ]) {
                Ok(_) => println!("Successfully linked Opus branch."),
                Err(err) => eprintln!("Failed to link Opus branch: {}", err),
            }
        } else if media_type == "video" {

            let sink_pad = queue_h264_clone
                .static_pad("sink")
                .expect("Failed to get sink pad from queue_h264_clone.");
            match src_pad.link(&sink_pad) {
                Ok(_) => println!("Linked video pad to queue_h264_clone."),
                Err(err) => {
                    eprintln!("Failed to link video pad: {}", err);
                    return;
                }
            }
    

            match gstreamer::Element::link_many(&[
                &queue_h264_clone,
                &rtp_h264_depay_clone,
                &h264_parse_clone,
                &mp4mux_h264_clone,
            ]) {
                Ok(_) => println!("Successfully linked H264 branch."),
                Err(err) => eprintln!("Failed to link H264 branch: {}", err),
            }
        } else {
            println!("Unknown media type: {}", media_type);
        }
    });
    

    let main_loop = glib::MainLoop::new(None, false);

    let main_loop_clone = main_loop.clone();

    let bus = pipeline.bus().unwrap();
    let _ = bus.add_watch(move |_, msg| {


        match msg.view() {
            MessageView::Eos(..) => {
                println!("End-Of-Stream reached.");
                main_loop_clone.quit();
            }
            MessageView::Error(err) => {
                eprintln!(
                    "Error received from {:?}: {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                eprintln!("Debugging information: {:?}", err.debug());
                main_loop_clone.quit();
            }
            _ => (),
        }
        glib::ControlFlow::Continue
    });

    pipeline.set_state(State::Playing)?;

    println!("Pipeline is running...");
    println!("Press Ctrl+C to quit.");

    let main_loop_clone_ctrlc = main_loop.clone();
    ctrlc::set_handler(move || {
        println!("Interrupt received, quitting...");
        main_loop_clone_ctrlc.quit();
    })?;

    main_loop.run();

    pipeline.set_state(State::Null)?;


    Ok(())

}

