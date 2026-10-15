//! Print the segment table of an SPK kernel: `cargo run -p kshana-ephem --example inspect -- data/de421.bsp`
use kshana_ephem::Spk;

fn main() {
    let path = std::env::args().nth(1).expect("path to .bsp");
    match Spk::open(std::path::Path::new(&path)) {
        Ok(spk) => {
            println!(
                "{:<7} {:<7} {:>5} {:>5} {:>18} {:>18}",
                "target", "center", "frame", "type", "start_et", "end_et"
            );
            for s in spk.segments() {
                println!(
                    "{:<7} {:<7} {:>5} {:>5} {:>18.1} {:>18.1}",
                    s.target, s.center, s.frame, s.data_type, s.start_et, s.end_et
                );
            }
            if let Some(et) = std::env::args().nth(2).and_then(|s| s.parse::<f64>().ok()) {
                use kshana_ephem::Body;
                for (t, c) in [
                    (Body::Earth, Body::Sun),
                    (Body::Moon, Body::Earth),
                    (Body::EarthMoonBarycenter, Body::SolarSystemBarycenter),
                ] {
                    match spk.state(t, c, et) {
                        Ok(st) => {
                            let r = (st.position[0].powi(2)
                                + st.position[1].powi(2)
                                + st.position[2].powi(2))
                            .sqrt();
                            println!("{t:?} rel {c:?}: pos {:?} |r| = {r:.3} km", st.position);
                        }
                        Err(e) => println!("{t:?} rel {c:?}: error {e:?}"),
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("error: {e:?}");
            std::process::exit(1);
        }
    }
}
