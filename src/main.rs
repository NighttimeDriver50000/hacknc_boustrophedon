extern crate image;
extern crate petgraph;

use image::ImageDecoder;
use image::png::PNGDecoder;

use petgraph::graph;

use std::env;
use std::error::Error;
use std::fs::File;
use std::vec::Vec;

struct Point {
    x: usize,
    y: usize,
}

struct LineRegion {
    obstacle: bool,
    start: usize, // Inclusive
    end: usize, // Exclusive
}

impl Clone for LineRegion {
    fn clone(&self) -> LineRegion {
        LineRegion {
            obstacle: self.obstacle,
            start: self.start,
            end: self.end,
        }
    }

    fn clone_from(&mut self, source: &LineRegion) {
        self.obstacle = source.obstacle;
        self.start = source.start;
        self.end = source.end;
    }
}

/// Return true if linear luminance is less than half full.
/// Linear luminance is (r + b + g) / 3.
fn is_obstacle_rgb(line: &Vec<u8>, x: usize) -> bool {
    (line[3*x] as u16) + (line[3*x+1] as u16) + (line[3*x+2] as u16) < 0x180
}

fn find_borders(regions_vectors: &Vec<Vec<LineRegion>>, w: usize,
                h: usize) -> (usize, usize, usize, usize) {
    // Find borders
    let mut state = 0; 
    let mut top: usize = 0;
    let mut bottom: usize = 0;
    let mut left: usize = w;
    let mut right: usize = w;
    for (y, regions) in regions_vectors.iter().enumerate() {
        if state == 0 {  // Working on top border
            for region in regions.iter() {
                if !region.obstacle {  // White
                    top = y;
                    state = 1;
                    break;
                }
            }
        } else if state == 1 {  // Working on sides
            if regions[0].obstacle {  // Black
                if regions.len() == 1 { // All black
                    state = 2;
                    continue;
                } else if regions[0].end < left {
                    left = regions[0].end;
                }
            } else {  // White
                left = 0;
            }
            let regions_last = match regions.last() {
                Some(r) => r,
                _ => panic!("Invalid region vector!"),
            };
            if regions_last.obstacle {  // Black
                let right_margin = w - regions_last.start;
                if right_margin < right {
                    right = right_margin;
                }
            } else {  // White
                right = 0;
            }
        } else {  // State 2: check for bottom
            bottom += 1;
            for region in regions.iter() {
                if !region.obstacle {  // White
                    bottom = 0;
                    state = 1;
                    break;
                }
            }
        }
    }
    //println!("{}, {}, {}, {}", top, bottom, left, right);
    (top, bottom, left, right)
}

fn overlap(region1: &LineRegion, region2: &LineRegion) -> bool {
    (region2.start <= region1.start && region1.start < region2.end)
        || (region1.start <= region2.start && region2.start < region1.end)
}

/// Find have any critical_points given line regions.
fn find_critical_points(regions_vectors: &mut Vec<Vec<LineRegion>>, w: usize,
                        h: usize) -> Vec<Point> {
    let (top, bottom, left, right) = find_borders(&regions_vectors, w, h);
    let mut prev_regions = vec![LineRegion {
        obstacle: true,
        start: 0,
        end: w - right,
    }];
    regions_vectors.push(vec![LineRegion {
        obstacle: true,
        start: left,
        end: w - right,
    }]);
    let mut crit_points = vec![];
    for regions in &mut regions_vectors[top..(1 + h - bottom)] {
        regions[0].start = left;
        if regions[0].start == regions[0].end {
            regions.remove(0);
        }
        let mut needs_pop = false;
        if let Some(regions_last) = regions.last_mut() {
            regions_last.end = w - right;
            if regions_last.start == regions_last.end {
                needs_pop = true;
            }
        }
        if needs_pop {
            regions.pop();
        }
    }
    for (y, regions) in (&regions_vectors[top..(1 + h - bottom)]).iter().enumerate() {
        let y = y + top;
        let prev_regions_clone = prev_regions.clone();
        let mut continuity: graph::Graph<&LineRegion, u8, petgraph::Undirected>
            = graph::Graph::new_undirected();
        let mut obstacles: Vec<graph::NodeIndex> = vec![];
        for region in regions {
            if region.obstacle {
                obstacles.push(continuity.add_node(region));
            }
        }
        for prev_region in prev_regions_clone.iter() {
            if prev_region.obstacle {
                let prev_region_id = continuity.add_node(prev_region);
                for region_id in obstacles.iter() {
                    if overlap(continuity[prev_region_id],
                               continuity[*region_id]) {
                        continuity.add_edge(prev_region_id, *region_id, 0);
                    }
                }
                let mut neighbors: Vec<&LineRegion> = vec![];
                for region_id in continuity.neighbors(prev_region_id) {
                    neighbors.push(continuity[region_id]);
                }
                if neighbors.len() == 0 {
                    crit_points.push(Point {
                        x: (prev_region.start + prev_region.end) / 2,
                        y: y,
                    });
                } else if neighbors.len() == 2 {
                    if neighbors[0].start > neighbors[1].start {
                        neighbors.swap(0, 1);
                    }
                    crit_points.push(Point {
                        x: (neighbors[0].end + neighbors[1].start) / 2,
                        y: y,
                    });
                } else if neighbors.len() > 2 {
                    panic!("Unanticipated edge case!");
                }
            }
        }
        for region_id in obstacles {
            let region = continuity[region_id];
            let mut neighbors: Vec<&LineRegion> = vec![];
            for prev_region_id in continuity.neighbors(region_id) {
                neighbors.push(continuity[prev_region_id]);
            }
            if neighbors.len() == 0 {
                crit_points.push(Point {
                    x: (region.start + region.end) / 2,
                    y: y - 1,
                });
            } else if neighbors.len() == 2 {
                if neighbors[0].start > neighbors[1].start {
                    neighbors.swap(0, 1);
                }
                crit_points.push(Point {
                    x: (neighbors[0].end + neighbors[1].start) / 2,
                    y: y - 1,
                });
            } else if neighbors.len() > 2 {
                panic!("Unanticipated edge case!");
            }
        }
        prev_regions = regions.clone();
    }
    crit_points
}

fn boustrophedon_png(filename: String) -> Result<String, Box<Error>> {
    let file = try!(File::open(filename));
    let mut decoder = PNGDecoder::new(file);
    let (w, h) = try!(decoder.dimensions());
    let row_len = try!(decoder.row_len());

    let mut line: Vec<u8> = vec![0; row_len];
    let mut regions_vectors: Vec<Vec<LineRegion>> = vec![];

    while let Ok(_) = decoder.read_scanline(&mut line[..]) {
        let mut regions: Vec<LineRegion> = vec![];
        let mut start: usize = 0;
        let mut prev_obstacle = false;
        let mut obstacle = is_obstacle_rgb(&line, 0);
        for x in 1..w as usize {
            prev_obstacle = obstacle;
            obstacle = is_obstacle_rgb(&line, x);
            if obstacle != prev_obstacle {
                regions.push(LineRegion {
                    obstacle: prev_obstacle,
                    start: start,
                    end: x,
                });
                start = x;
            }
        }
        regions.push(LineRegion {
            obstacle: prev_obstacle,
            start: start,
            end: w as usize,
        });
        /*
        for region in &regions {
            print!("{},{},{}; ", region.obstacle, region.start, region.end);
        }
        println!("");
        */
        regions_vectors.push(regions);
    }
    let crit_points = find_critical_points(&mut regions_vectors,
                                           w as usize, h as usize);
    println!("[");
    for (i, crit_point) in crit_points.iter().enumerate() {
        print!("  [{}, {}]", crit_point.x, crit_point.y);
        if i != crit_points.len() - 1 {
            println!(",");
        }
    }
    println!("\n]");
    Ok("Done".into())
}

fn main() {
    let filename: String;
    match env::args().nth(1) {
        Some(name) => filename = name,
        None => panic!("No image provided!"),
    }
    match boustrophedon_png(filename) {
        Ok(s)  => (),//println!("{}", s),
        Err(e) => println!("Error: {}", e.to_string()),
    }
}
