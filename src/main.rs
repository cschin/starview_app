// Entry point for non-wasm
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    run().await;
}

use anyhow::Result;
use fxhash::FxHashMap;

#[cfg(feature = "csv_data")]
use csv;
#[cfg(feature = "hdf5_data")]
use hdf5;
use std::path::Path;

use std::io::{BufRead, BufReader};

use ndarray::{arr2, s, Array2};
use three_d::{egui::style::Selection, *};
type GeneExpression = FxHashMap<String, Vec<f32>>;

#[cfg(feature = "hdf5_data")]
fn read_cell_locations() -> Result<(Array2<u64>, GeneExpression)> {
    let file = hdf5::File::open("test_data/cortex_starmap.h5")?; // open for reading
    let ds = file.dataset("obsm/spatial3D")?; // open the dataset
    let positions = ds.read_slice_2d::<u64, _>(hdf5::Selection::All)?;
    let ds = file.dataset("X/data")?;
    let ge_value = ds.read_1d::<f32>()?;
    let ds = file.dataset("var/_index")?;
    let ge_labels = ds.read_1d::<hdf5::types::VarLenUnicode>()?;
    let ge_labels = ge_labels
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
    let ds = file.dataset("X/indices")?;
    let ge_index = ds.read_1d::<u32>()?;
    let mut ge = GeneExpression::default();
    ge_index
        .into_iter()
        .zip(ge_value.into_iter())
        .for_each(|(idx, v)| {
            ge.entry(ge_labels[idx as usize].clone())
                .or_insert_with(Vec::new)
                .push(v);
        });

    Ok((positions, ge))
}

#[cfg(feature = "csv_data")]
fn read_cell_locations() -> Result<(Array2<u64>, GeneExpression)> {
    use std::fs::File;

    use anyhow::Error;

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(Path::new("test_data/STARmap_3D_data_cell_locations.txt"))?;

    let mut positions = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let pt = [
            record[0].parse::<u64>()?,
            record[1].parse::<u64>()?,
            record[2].parse::<u64>()?,
        ];
        positions.push(pt);
    }
    let positions = arr2(&positions);

    let mut ge = GeneExpression::default();
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_path(Path::new("test_data/STARmap_3D_data_expression.txt"))?;
    rdr.records().into_iter().for_each(|r| {
        if let Ok(r) = r {
            let ge_label = r[0].to_string();
            let v = (1..r.len())
                .map(|idx| r[idx].parse::<f32>().expect("parsing error"))
                .collect::<Vec<_>>();
            ge.insert(ge_label, v);
        }
    });

    Ok((positions, ge))
}

pub async fn run() {
    let window = Window::new(WindowSettings {
        title: "Simple 3D Spatial Viewer".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(1.0, 1.0, 1.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 0.0, 1.0),
        degrees(45.0),
        0.001,
        1000.0,
    );
    let mut control = OrbitControl::new(*camera.target(), 0.1, 1.0);

    let output = if let Ok((cell_locations, expression)) = read_cell_locations() {
        let scaling_factor = 0.0005f32;
        let positions = Positions::F32(
            cell_locations
                .outer_iter()
                .map(|v| {
                    vec3(
                        v[0] as f32 * scaling_factor,
                        v[1] as f32 * scaling_factor,
                        v[2] as f32 * scaling_factor,
                    )
                })
                .collect::<Vec<_>>(),
        );
        let c = Srgba {
            r: 149,
            g: 200,
            b: 10,
            a: 120,
        };
        let colors = Some((0..positions.len()).map(|_| c).collect::<Vec<_>>());
        (PointCloud { positions, colors }, expression)
    } else {
        panic!("can't read the input file")
    };
    let mut cpu_point_cloud = output.0;
    let n_points = cpu_point_cloud.positions.len();
    let expression = output.1;

    let mut e_keys = expression.keys().map(|v| v.clone()).collect::<Vec<_>>();
    let mut e_states = e_keys.iter().map(|_| false).collect::<Vec<_>>();

    println!("{:?}", cpu_point_cloud);

    let mut point_mesh = CpuMesh::sphere(6);
    point_mesh.transform(&Mat4::from_scale(0.002)).unwrap();

    let mut point_cloud = Gm {
        geometry: InstancedMesh::new(&context, &cpu_point_cloud.clone().into(), &point_mesh),
        material: ColorMaterial::new_transparent(&context, &CpuMaterial::default()),
    };

    let c = -point_cloud.aabb().center();
    point_cloud.set_transformation(Mat4::from_translation(c));

    let mut gui = three_d::GUI::new(&context);

    let mut threshold = 1250.0;
    let mut last_threshold = threshold;

    // main loop
    let mut last_states = Vec::new();
    window.render_loop(move |mut frame_input| {
        let mut panel_width = 0.0;
        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |gui_context| {
                use three_d::egui::*;
                SidePanel::left("side_panel")
                    .resizable(true)
                    .show(gui_context, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.heading("Select");
                            ui.add(Slider::new(&mut threshold, 0.0..=10000.0).text("Threshold"));
                            //ui.add(Checkbox::new());
                            e_keys.iter().enumerate().for_each(|(idx, label)| {
                                ui.add(Checkbox::new(&mut e_states[idx], label.clone()));
                            });
                            // Add a lot of widgets here.
                        });
                    });

                panel_width = gui_context.used_rect().width();
            },
        );
        let viewport = Viewport {
            x: (panel_width * frame_input.device_pixel_ratio) as i32,
            y: 0,
            width: frame_input.viewport.width
                - (panel_width * frame_input.device_pixel_ratio) as u32,
            height: frame_input.viewport.height,
        };
        let mut redraw = frame_input.first_frame;
        camera.set_viewport(viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        let c1 = Srgba {
            r: 255,
            g: 140,
            b: 10,
            a: 180,
        };

        let c2 = Srgba {
            r: 200,
            g: 200,
            b: 200,
            a: 50,
        };

        let diff: f32 = threshold - last_threshold;
        let mut lift_points = Vec::new();
        if last_states != e_states || diff.abs() > 1.0 {
            let colors = Some(
                (0..n_points)
                    .map(|idx| {
                        let mut selected = false;
                        e_keys.iter().zip(e_states.iter()).for_each(|(k, s)| {
                            if *s {
                                let v = expression.get(k).unwrap();
                                if idx < v.len() {
                                    let v = *v.get(idx as usize).unwrap();
                                    if v > threshold {
                                        selected = true;
                                    }
                                }
                            }
                        });
                        if selected {
                            lift_points.push(idx);
                            c1
                        } else {
                            c2
                        }
                    })
                    .collect::<Vec<_>>(),
            );

            // let mut cpu_point_cloud = cpu_point_cloud0.clone();
            // for idx in lift_points {
            //     if let Positions::F32(p) = &mut cpu_point_cloud.positions {
            //         let p = &mut p[idx];
            //         p[2] += 0.025;
            //     };
            // }
            cpu_point_cloud.colors = colors;

            point_cloud = Gm {
                geometry: InstancedMesh::new(
                    &context,
                    &cpu_point_cloud.clone().into(),
                    &point_mesh,
                ),
                material: ColorMaterial::new_transparent(&context, &CpuMaterial::default()),
            };
            let c = -point_cloud.aabb().center();
            point_cloud.set_transformation(Mat4::from_translation(c));
            last_states = e_states.clone();
            last_threshold = threshold;
        }

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
            .render(&camera, point_cloud.into_iter(), &[]);

        gui.render();

        FrameOutput::default()
    });
}
