use vergen_gix::{BuildBuilder, CargoBuilder, DependencyKind, GixBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_build_details()?;

    assert!(
        std::path::Path::new("../../frontend/dist").is_dir(),
        "The frontend needs to be build first!"
    );

    Ok(())
}

fn generate_build_details() -> Result<(), Box<dyn std::error::Error>> {
    let mut emitter = vergen_gix::Emitter::default();

    emitter.add_instructions(&BuildBuilder::all_build()?)?;
    emitter.add_instructions(
        CargoBuilder::all_cargo()?.set_dep_kind_filter(Some(DependencyKind::Normal)),
    )?;

    if std::path::Path::new("../../.git").is_dir() {
        emitter.add_instructions(&GixBuilder::all_git()?)?;
    }

    emitter.emit()?;

    Ok(())
}
