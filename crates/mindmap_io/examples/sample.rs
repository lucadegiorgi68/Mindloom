use mindmap_core::{MindMap, Shape};
fn main() {
    let mut m = MindMap::new("Atom structure");
    let nucleus = m.add_child(1, "Nucleus").unwrap();
    let electrons = m.add_child(1, "Electrons").unwrap();
    let history = m.add_child(1, "History").unwrap();
    let models = m.add_child(1, "Models").unwrap();
    let forces = m.add_child(1, "Forces").unwrap();
    for t in ["Protons", "Neutrons", "Quarks"] {
        m.add_child(nucleus, t).unwrap();
    }
    let sh = m.add_child(electrons, "Shells").unwrap();
    m.add_child(sh, "K shell").unwrap();
    m.add_child(sh, "L shell").unwrap();
    m.add_child(electrons, "Orbitals").unwrap();
    m.add_child(electrons, "Spin").unwrap();
    for t in ["Dalton", "Thomson", "Rutherford", "Bohr"] {
        m.add_child(history, t).unwrap();
    }
    m.add_child(models, "Plum pudding").unwrap();
    m.add_child(models, "Quantum model").unwrap();
    m.add_child(forces, "Strong").unwrap();
    m.add_child(forces, "Electromagnetic").unwrap();
    m.nodes.get_mut(&forces).unwrap().shape = Shape::Cloud;
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).map(String::as_str).unwrap_or("sample.svg");
    std::fs::write(out, mindmap_io::to_svg(&m)).unwrap();
    println!("wrote {out}");
}
