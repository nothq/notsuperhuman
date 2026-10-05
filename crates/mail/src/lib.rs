#![forbid(unsafe_code)]

pub mod live;
pub mod model;
pub mod ui;

#[cfg(test)]
mod test;

/// The mail surface for every account signed in on this machine.
pub fn production_root() -> ui::SurfaceRoot {
    ui::SurfaceRoot::production(
        live::production_mail_bootstrap_api(),
        live::production_mail_local_file_api(),
    )
}
