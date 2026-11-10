use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

mod entities;
mod schema;
mod update;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAIN_TEMPLATE: &str = "use fragment_rs::FragmentApp;\n\n#[tokio::main]\nasync fn main() {\n    // Controllers and services register themselves, so there is nothing to wire up.\n    // The address and the log level come from `fragment.toml`.\n    FragmentApp::from_config_file_or_default(\"fragment.toml\")\n        .start()\n        .await;\n}\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Structure {
    Layered,
    Module,
    Feature,
}

impl Structure {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "layered" => Ok(Self::Layered),
            "module" => Ok(Self::Module),
            "feature" => Ok(Self::Feature),
            _ => Err(format!(
                "unknown structure `{value}`; choose layered, module, or feature"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Layered => "layered",
            Self::Module => "module",
            Self::Feature => "feature",
        }
    }

    fn domain_root(self) -> &'static str {
        match self {
            Self::Module => "modules",
            Self::Feature => "features",
            Self::Layered => "",
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(env::args().skip(1).collect()).await {
        eprintln!("frs: {error}");
        process::exit(1);
    }
}

async fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    match command {
        "--help" | "-h" | "help" => print_help(),
        "--version" | "-V" | "version" => println!("frs {VERSION}"),
        "create" | "new" => create_project(&args[1..])?,
        "make:mod" | "make:module" => make_module(&args[1..])?,
        "make:controller" => make_component(&args[1..], Component::Controller)?,
        "make:service" => make_component(&args[1..], Component::Service)?,
        "make:entity" => make_component(&args[1..], Component::Entity)?,
        "make:migration" => make_migration(&args[1..]).await?,
        "add" => add_item(&args[1..])?,
        "check" => run_check(&args[1..])?,
        "doctor" => doctor(),
        "update" => update::update(&args[1..])?,
        unknown => return Err(format!("unknown command `{unknown}`. Run `frs help`.")),
    }
    Ok(())
}

fn print_help() {
    println!(
        "Fragment Rust CLI ({VERSION})\n\
         \n\
         USAGE:\n\
         \x20 frs <COMMAND> [OPTIONS]\n\
         \n\
         PROJECT COMMANDS:\n\
         \x20 create <name> [--structure layered|module|feature] [--path DIR]\n\
         \x20 make:mod <name> [--pmod PARENT]\n\
         \x20 make:controller <name> [--pmod PARENT]\n\
         \x20 make:service <name> [--pmod PARENT]\n\
         \x20 make:entity <name>\n\
         \x20 make:migration <name> [-g]\n\
         \x20 check [--path DIR]                 Run cargo check\n\
         \x20 doctor                             Check Rust tooling\n\
         \x20 update [--check]                   Update the CLI to the latest release\n\
         \x20 help                               Show this help\n\
         \x20 version                            Show CLI version\n\
         \n\
         EXAMPLES:\n\
         \x20 frs create storefront --structure module\n\
         \x20 cd storefront && frs make:mod authentication\n\
         \x20 frs make:mod tokens --pmod=authentication\n\
         \x20 frs make:controller login --pmod=authentication\n\
         \x20 frs make:entity user\n\
         \x20 frs make:migration create_users_table\n\
         \x20 frs make:migration -g --database-url postgres://localhost/app\n\
         \x20 frs check\n\
         \x20 frs update"
    );
}

fn create_project(args: &[String]) -> Result<(), String> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!("Usage: frs create <name> [--structure layered|module|feature] [--path DIR]");
        return Ok(());
    }
    let mut name = None;
    let mut parent = PathBuf::from(".");
    let mut structure = Structure::Module;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--path" || arg == "-p" {
            index += 1;
            parent = PathBuf::from(args.get(index).ok_or("`--path` requires a directory")?);
        } else if arg == "--structure" || arg == "-s" {
            index += 1;
            structure = Structure::parse(args.get(index).ok_or("`--structure` requires a value")?)?;
        } else if let Some(value) = arg.strip_prefix("--structure=") {
            structure = Structure::parse(value)?;
        } else if arg.starts_with('-') {
            return Err(format!("unknown option `{arg}` for `create`"));
        } else if name.is_none() {
            name = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument `{arg}` for `create`"));
        }
        index += 1;
    }
    let name = name.ok_or("missing project name. Usage: frs create <name>")?;
    validate_package_name(&name)?;
    let destination = parent.join(&name);
    scaffold_project(&destination, &name, structure)?;
    println!(
        "Created Fragment application `{name}` ({}) at {}",
        structure.as_str(),
        destination.display()
    );
    println!("\nNext steps:\n  cd {}\n  cargo run", destination.display());
    Ok(())
}

fn scaffold_project(destination: &Path, name: &str, structure: Structure) -> Result<(), String> {
    if destination.exists() {
        return Err(format!(
            "`{}` already exists; refusing to overwrite it",
            destination.display()
        ));
    }
    fs::create_dir_all(destination.join("src"))
        .map_err(|error| format!("could not create project directory: {error}"))?;
    let result = scaffold_layout(destination, structure)
        .and_then(|()| write_file(&destination.join("Cargo.toml"), &project_manifest(name)))
        .and_then(|()| {
            write_file(
                &destination.join("README.md"),
                &project_readme(name, structure),
            )
        })
        .and_then(|()| {
            write_file(
                &destination.join("fragment.toml"),
                &project_config(structure),
            )
        });
    if let Err(error) = result {
        let _ = fs::remove_dir_all(destination);
        return Err(error);
    }
    Ok(())
}

fn scaffold_layout(root: &Path, structure: Structure) -> Result<(), String> {
    write_file(&root.join("src/main.rs"), MAIN_TEMPLATE)?;
    match structure {
        Structure::Layered => {
            for folder in ["controllers", "services", "repositories", "entities", "dto"] {
                write_module_directory(&root.join("src").join(folder), None)?;
            }
            write_module_directory(&root.join("src/common"), None)?;
            write_component_file(
                &root.join("src/controllers/app_controller.rs"),
                "AppController",
                Component::Controller,
            )?;
            append_module(&root.join("src/controllers/mod.rs"), "app_controller")?;
            write_component_file(
                &root.join("src/services/app_service.rs"),
                "AppService",
                Component::Service,
            )?;
            append_module(&root.join("src/services/mod.rs"), "app_service")?;
            append_module(&root.join("src/main.rs"), "controllers")?;
            append_module(&root.join("src/main.rs"), "services")?;
            append_module(&root.join("src/main.rs"), "repositories")?;
            append_module(&root.join("src/main.rs"), "entities")?;
            append_module(&root.join("src/main.rs"), "dto")?;
            append_module(&root.join("src/main.rs"), "common")?;
        }
        Structure::Module => {
            write_module_directory(&root.join("src/modules"), None)?;
            write_module_directory(&root.join("src/entities"), None)?;
            append_module(&root.join("src/main.rs"), "modules")?;
            append_module(&root.join("src/main.rs"), "entities")?;
            create_domain_skeleton(&root.join("src/modules"), "app")?;
            append_module(&root.join("src/modules/mod.rs"), "app")?;
        }
        Structure::Feature => {
            write_module_directory(&root.join("src/features"), None)?;
            write_module_directory(&root.join("src/shared"), None)?;
            for folder in ["guards", "filters", "middleware"] {
                write_module_directory(&root.join("src/shared").join(folder), None)?;
            }
            write_module_directory(&root.join("src/entities"), None)?;
            append_module(&root.join("src/main.rs"), "features")?;
            append_module(&root.join("src/main.rs"), "shared")?;
            append_module(&root.join("src/main.rs"), "entities")?;
            create_domain_skeleton(&root.join("src/features"), "app")?;
            append_module(&root.join("src/features/mod.rs"), "app")?;
        }
    }
    Ok(())
}

fn project_config(structure: Structure) -> String {
    format!(
        "[structure]\ntype = \"{}\"\n\n[server]\nhost = \"0.0.0.0\"\nport = 3000\n",
        structure.as_str()
    )
}

fn project_readme(name: &str, structure: Structure) -> String {
    format!(
        "# {name}\n\nA Fragment Rust application using the **{}** project structure.\n\n## Run locally\n\n```sh\ncargo run\n```\n\n## Generate a module\n\n```sh\nfrs make:mod authentication\n```\n\nSee the Fragment Rust documentation for project layouts and CLI usage.\n",
        structure.as_str()
    )
}

fn make_module(args: &[String]) -> Result<(), String> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!("Usage: frs make:mod <name> [--pmod PARENT]\nCreates a domain module under src/modules (module layout) or src/features (feature layout). Parent paths may use `/`.");
        return Ok(());
    }
    let (name, parents) = parse_name_and_parent(args, "make:mod")?;
    let module = to_snake_case(&name);
    validate_identifier(&module)?;
    let root = project_root()?;
    let structure = read_structure(&root)?;
    if structure == Structure::Layered {
        return Err("`make:mod` is for module/feature layouts. In layered projects use `frs make:controller` and `frs make:service`.".to_owned());
    }
    let domain_root = structure.domain_root();
    let domain_path = root.join("src").join(domain_root);
    ensure_root_module(&root, domain_root)?;
    let mut parent_path = domain_path.clone();
    for parent in parents {
        let parent = to_snake_case(&parent);
        validate_identifier(&parent)?;
        let next = parent_path.join(&parent);
        ensure_domain_skeleton(&root, &parent_path, &next, &parent)?;
        parent_path = next;
    }
    if parent_path.join(&module).exists() {
        return Err(format!(
            "module `{module}` already exists at {}",
            parent_path.join(&module).display()
        ));
    }
    create_domain_skeleton(&parent_path, &module)?;
    append_module(&parent_path.join("mod.rs"), &module)?;
    println!(
        "Created {module} module in {} layout under {}",
        structure.as_str(),
        parent_path.display()
    );
    Ok(())
}

fn ensure_root_module(root: &Path, domain_root: &str) -> Result<(), String> {
    let dir = root.join("src").join(domain_root);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    let main_file = root.join("src/main.rs");
    append_module(&main_file, domain_root)
}

fn ensure_domain_skeleton(
    root: &Path,
    parent: &Path,
    target: &Path,
    name: &str,
) -> Result<(), String> {
    if !target.exists() {
        create_domain_skeleton(parent, name)?;
    }
    let relative = target
        .strip_prefix(root.join("src"))
        .map_err(|_| "parent module path is outside src".to_owned())?;
    let mut declaration_file = root.join("src");
    for part in relative
        .iter()
        .take(relative.components().count().saturating_sub(1))
    {
        declaration_file.push(part);
    }
    declaration_file.push("mod.rs");
    append_module(&declaration_file, name)
}

fn create_domain_skeleton(parent: &Path, module: &str) -> Result<(), String> {
    let module_dir = parent.join(module);
    write_module_directory(&module_dir, Some(module))?;
    for kind in ["controllers", "services"] {
        write_module_directory(&module_dir.join(kind), None)?;
        append_module(&module_dir.join("mod.rs"), kind)?;
    }
    let snake = to_snake_case(module);
    let pascal = to_pascal_case(module);
    write_component_file(
        &module_dir.join(format!("controllers/{snake}_controller.rs")),
        &format!("{pascal}Controller"),
        Component::Controller,
    )?;
    append_module(
        &module_dir.join("controllers/mod.rs"),
        &format!("{snake}_controller"),
    )?;
    write_component_file(
        &module_dir.join(format!("services/{snake}_service.rs")),
        &format!("{pascal}Service"),
        Component::Service,
    )?;
    append_module(
        &module_dir.join("services/mod.rs"),
        &format!("{snake}_service"),
    )?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Component {
    Controller,
    Service,
    Entity,
}

impl Component {
    /// The directory this component belongs to, relative to a module or feature root.
    fn category(self) -> &'static str {
        match self {
            Self::Controller => "controllers",
            Self::Service => "services",
            Self::Entity => "entities",
        }
    }
}

fn make_component(args: &[String], component: Component) -> Result<(), String> {
    let command = match component {
        Component::Controller => "make:controller",
        Component::Service => "make:service",
        Component::Entity => "make:entity",
    };
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!("Usage: frs {command} <name> [--pmod PARENT]");
        return Ok(());
    }
    let (name, parents) = parse_name_and_parent(args, command)?;
    let snake = to_snake_case(&name);
    validate_identifier(&snake)?;
    let root = project_root()?;
    let structure = read_structure(&root)?;
    let category = component.category();
    let mut base = root.join("src");
    match structure {
        Structure::Layered => base.push(category),
        Structure::Module => {
            base.push("modules");
        }
        Structure::Feature => {
            base.push("features");
        }
    }
    let parents = if parents.is_empty()
        && structure != Structure::Layered
        && component != Component::Entity
    {
        vec!["app".to_owned()]
    } else {
        parents
    };
    for parent in &parents {
        let parent = to_snake_case(parent);
        validate_identifier(&parent)?;
        base.push(parent);
    }
    if structure != Structure::Layered && component != Component::Entity {
        base.push(category);
    } else if structure != Structure::Layered && component == Component::Entity {
        base = root.join("src/entities");
    }
    fs::create_dir_all(&base)
        .map_err(|error| format!("could not create {}: {error}", base.display()))?;
    ensure_module_chain(&root.join("src"), &base)?;
    let type_name = to_pascal_case(&snake);
    let (file_name, declaration, type_name) = match component {
        Component::Controller => (
            format!("{snake}_controller.rs"),
            format!("{snake}_controller"),
            format!("{type_name}Controller"),
        ),
        Component::Service => (
            format!("{snake}_service.rs"),
            format!("{snake}_service"),
            format!("{type_name}Service"),
        ),
        Component::Entity => (format!("{snake}.rs"), snake.clone(), type_name),
    };
    let destination = base.join(file_name);
    if destination.exists() {
        return Err(format!(
            "{} already exists; refusing to overwrite it",
            destination.display()
        ));
    }
    write_component_file(&destination, &type_name, component)?;
    append_module(&base.join("mod.rs"), &declaration)?;
    if component == Component::Entity {
        // SeaORM's derive macros expand to `sea_orm::` paths, so the entity's
        // crate has to depend on sea-orm directly, not only through fragment-rs.
        add_cargo_dependency(
            &root,
            "sea-orm",
            "version = \"1.1\", features = [\"macros\", \"runtime-tokio-rustls\", \"sqlx-postgres\", \"sqlx-mysql\", \"sqlx-sqlite\"]",
        )?;
    }
    println!(
        "Generated {} at {}",
        command.trim_start_matches("make:"),
        destination.display()
    );
    Ok(())
}

fn ensure_module_chain(src: &Path, target: &Path) -> Result<(), String> {
    let relative = target
        .strip_prefix(src)
        .map_err(|_| "generated path is outside src".to_owned())?;
    let mut current = src.to_path_buf();
    let parts: Vec<_> = relative.iter().collect();
    for (index, part) in parts.iter().enumerate() {
        let parent_mod = if index == 0 {
            crate_root_file(src)
        } else {
            current.join("mod.rs")
        };
        append_module(&parent_mod, &part.to_string_lossy())?;
        current.push(part);
        if index + 1 < parts.len() {
            fs::create_dir_all(&current)
                .map_err(|error| format!("could not create {}: {error}", current.display()))?;
        }
    }
    Ok(())
}

/// The file that declares the crate's top-level modules.
///
/// A generated project is a binary crate, so the root is `src/main.rs`. Projects
/// that expose a library use `src/lib.rs` instead.
fn crate_root_file(src: &Path) -> PathBuf {
    let lib = src.join("lib.rs");
    if lib.is_file() {
        lib
    } else {
        src.join("main.rs")
    }
}

fn write_component_file(path: &Path, name: &str, component: Component) -> Result<(), String> {
    let contents = match component {
        Component::Controller => controller_template(name),
        Component::Service => service_template(name),
        Component::Entity => entity_template(name, &to_snake_case(name)),
    };
    write_file(path, &contents)
}

/// Generates a controller that already compiles and shows the routing API.
fn controller_template(name: &str) -> String {
    let snake = to_snake_case(name);
    format!(
        "use axum::Json;\nuse fragment_rs::{{controller, route, GET, POST}};\nuse serde::{{Deserialize, Serialize}};\n\n/// HTTP controller for the {name} use case.\n///\n/// `#[controller(\"/api/{snake}\")]` sets the prefix for every route in this\n/// block. The routes are registered automatically when the app starts.\npub struct {name};\n\n#[controller(\"/api/{snake}\")]\nimpl {name} {{\n    /// `GET /api/{snake}`\n    #[route(GET, \"/\")]\n    async fn list() -> Json<Vec<{name}Response>> {{\n        Json(Vec::new())\n    }}\n\n    /// `POST /api/{snake}`\n    #[route(POST, \"/\")]\n    async fn create(payload: Json<{name}Response>) -> Json<{name}Response> {{\n        Json(payload.0)\n    }}\n}}\n\n/// Request and response body for the {name} use case.\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name}Response {{\n    pub id: String,\n    pub name: String,\n}}\n"
    )
}

/// Generates a service that already compiles and shows the injection API.
fn service_template(name: &str) -> String {
    format!(
        "use fragment_rs::service;\n\n/// Business logic for the {name} use case.\n///\n/// Services are singletons: `#[service]` makes the type injectable, so a\n/// controller can request it with `Inject<{name}>`.\n///\n/// Deriving `Default` lets the service register itself, so the application\n/// finds it without being told about it.\n#[derive(Debug, Default)]\n#[service]\npub struct {name};\n\nimpl {name} {{\n    /// Implement the behaviour of this use case here.\n    pub fn execute(&self) -> String {{\n        String::from(\"{name}\")\n    }}\n}}\n"
    )
}

/// Options accepted by `frs make:migration`.
struct MigrationOptions {
    name: Option<String>,
    generate: bool,
    database_url: Option<String>,
    database: Option<String>,
    entities: Option<String>,
}

async fn make_migration(args: &[String]) -> Result<(), String> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: frs make:migration <name> [-g|--generate] [options]\n\
             \n\
             Creates a SeaORM migration inside the project's `migrations/` crate.\n\
             The migration crate is created on first use and registered as a workspace member.\n\
             \n\
             With `-g`, the migration is generated from the project's entity\n\
             definitions compared against the live database schema.\n\
             \n\
             Options:\n\
             \x20 -g, --generate            Generate the migration from entities and database state\n\
             \x20 --database-url URL       Database to compare against\n\
             \x20 --database NAME          Named database from `fragment.toml`\n\
             \x20 --entities DIR           Directory holding the entity definitions\n\
             \n\
             The database URL is read from `--database-url`, then the DATABASE_URL\n\
             environment variable, then the named database in `fragment.toml`.\n\
             \n\
             Generating a migration only reads the database. It never changes the\n\
             database and never applies a migration."
        );
        return Ok(());
    }

    let options = parse_migration_options(args)?;
    let root = project_root()?;

    if options.generate {
        return generate_migration(&root, &options).await;
    }

    let Some(raw_name) = options.name else {
        return Err("usage: frs make:migration <name>".to_owned());
    };
    let name = to_snake_case(&raw_name);
    validate_identifier(&name)?;
    write_migration_file(&root, &name, &migration_template(&name))
}

/// Parses the arguments of `frs make:migration`.
fn parse_migration_options(args: &[String]) -> Result<MigrationOptions, String> {
    let mut options = MigrationOptions {
        name: None,
        generate: false,
        database_url: None,
        database: None,
        entities: None,
    };

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "-g" | "--generate" => options.generate = true,
            "--database-url" => {
                index += 1;
                options.database_url = Some(
                    args.get(index)
                        .ok_or("`--database-url` requires a URL")?
                        .clone(),
                );
            }
            "--database" => {
                index += 1;
                options.database = Some(
                    args.get(index)
                        .ok_or("`--database` requires a database name")?
                        .clone(),
                );
            }
            "--entities" => {
                index += 1;
                options.entities = Some(
                    args.get(index)
                        .ok_or("`--entities` requires a directory")?
                        .clone(),
                );
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}` for `make:migration`"));
            }
            other => {
                if options.name.is_some() {
                    return Err(format!(
                        "unexpected argument `{other}` for `make:migration`"
                    ));
                }
                options.name = Some(other.to_owned());
            }
        }
        index += 1;
    }
    Ok(options)
}

/// Generates a migration from the entity definitions and the live schema.
async fn generate_migration(root: &Path, options: &MigrationOptions) -> Result<(), String> {
    let entities_dir = match &options.entities {
        Some(dir) => root.join(dir),
        None => root.join("src/entities"),
    };
    if !entities_dir.is_dir() {
        return Err(format!(
            "no entity directory at {}. Use --entities to point at one.",
            entities_dir.display()
        ));
    }

    let entities = entities::collect_entities(&entities_dir)?;
    if entities.is_empty() {
        return Err(format!(
            "no entity definitions found in {}. Add an entity with `frs make:entity` first.",
            entities_dir.display()
        ));
    }

    let url = resolve_database_url(root, options)?;
    println!("Reading the schema of {url}");
    let database = schema::introspect(&url).await?;
    let difference = schema::diff(&entities, &database);

    for note in &difference.notes {
        println!("note: {note}");
    }

    if difference.changes.is_empty() {
        println!("The database already matches the entity definitions; no migration was written.");
        return Ok(());
    }

    let name = match &options.name {
        Some(name) => to_snake_case(name),
        None => describe_changes(&difference.changes),
    };
    validate_identifier(&name)?;
    let contents = schema::render_migration(&difference.changes, &name);
    write_migration_file(root, &name, &contents)
}

/// Describes a set of changes with a readable migration name.
fn describe_changes(changes: &[schema::Change]) -> String {
    match changes {
        [schema::Change::CreateTable(entity)] => format!("create_{}", entity.table),
        [schema::Change::AddColumn { table, column }] => {
            format!("add_{table}_{}", column.name)
        }
        _ => "sync_entities".to_owned(),
    }
}

/// Finds the database to compare the entities against.
fn resolve_database_url(root: &Path, options: &MigrationOptions) -> Result<String, String> {
    if let Some(url) = &options.database_url {
        return Ok(url.clone());
    }
    if let Ok(url) = env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }
    if let Some(name) = &options.database {
        return named_database_url(root, name);
    }
    Err(
        "no database selected. Pass --database-url, set DATABASE_URL, or use --database <name>."
            .to_owned(),
    )
}

/// Reads a named database URL out of `fragment.toml`.
fn named_database_url(root: &Path, name: &str) -> Result<String, String> {
    let config = root.join("fragment.toml");
    let contents = fs::read_to_string(&config)
        .map_err(|error| format!("could not read {}: {error}", config.display()))?;

    let value: toml::Value = contents
        .parse()
        .map_err(|error| format!("could not parse {}: {error}", config.display()))?;

    let databases = value
        .get("databases")
        .and_then(|databases| databases.get(name))
        .ok_or_else(|| format!("`{name}` is not defined under [databases] in fragment.toml"))?;

    databases
        .get("url")
        .and_then(|url| url.as_str())
        .map(|url| url.to_owned())
        .ok_or_else(|| format!("database `{name}` in fragment.toml has no `url`"))
}

/// Writes a migration file into the project's migration crate.
fn write_migration_file(root: &Path, name: &str, contents: &str) -> Result<(), String> {
    let crate_name = migration_crate_name(root)?;
    let crate_dir = root.join("migrations");
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|error| format!("could not create {}: {error}", src_dir.display()))?;

    // The crate itself only needs to be written once; every migration adds a file to it.
    if !crate_dir.join("Cargo.toml").is_file() {
        write_file(
            &crate_dir.join("Cargo.toml"),
            &migration_manifest(&crate_name),
        )?;
        write_file(
            &src_dir.join("main.rs"),
            &migration_bin_template(&crate_name),
        )?;
        register_workspace_member(root, "migrations")?;
        add_cargo_dependency(root, &crate_name, "path = \"migrations\"")?;
    }

    let sequence = next_migration_sequence(&src_dir)?;
    let file_name = format!("m{sequence:04}_{name}");
    let file = src_dir.join(format!("{file_name}.rs"));
    if file.exists() {
        return Err(format!(
            "{} already exists; refusing to overwrite it",
            file.display()
        ));
    }
    write_file(&file, contents)?;
    regenerate_migration_lib(&src_dir)?;

    println!("Created migration {file_name} in {}", crate_dir.display());
    println!("Run it with: cargo run -p {crate_name} -- up");
    Ok(())
}

/// Derives the migration crate name from the application package name.
fn migration_crate_name(root: &Path) -> Result<String, String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read Cargo.toml: {error}"))?;
    let package = manifest
        .lines()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("name")
                .and_then(|rest| rest.split_once('='))
                .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        })
        .ok_or("could not find the package name in Cargo.toml")?;
    Ok(format!("{package}-migrations"))
}

/// Reports whether a file name follows the `mNNNN_name.rs` migration pattern.
///
/// `main.rs` and `lib.rs` live in the same directory and must not be mistaken
/// for migrations.
fn is_migration_file(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('m') else {
        return false;
    };
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && rest[digits..].starts_with('_') && name.ends_with(".rs")
}

/// Finds the next free migration sequence number in the migration crate.
fn next_migration_sequence(src_dir: &Path) -> Result<u32, String> {
    let mut highest = 0;
    for entry in fs::read_dir(src_dir)
        .map_err(|error| format!("could not read {}: {error}", src_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("could not read a directory entry: {error}"))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !is_migration_file(&file_name) {
            continue;
        }
        let rest = &file_name[1..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(value) = digits.parse::<u32>() {
            highest = highest.max(value);
        }
    }
    Ok(highest + 1)
}

/// Regenerates the migration crate's `lib.rs` from the migration files on disk.
///
/// Deriving the file list keeps the module declarations and the `Migrator`
/// migration list in sync without fragile string surgery.
fn regenerate_migration_lib(src_dir: &Path) -> Result<(), String> {
    let mut modules: Vec<String> = fs::read_dir(src_dir)
        .map_err(|error| format!("could not read {}: {error}", src_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_migration_file(name))
        .map(|name| name.trim_end_matches(".rs").to_owned())
        .collect();
    modules.sort();

    let declarations = modules
        .iter()
        .map(|module| format!("pub mod {module};"))
        .collect::<Vec<_>>()
        .join("\n");
    let entries = modules
        .iter()
        .map(|module| format!("            Box::new({module}::Migration),"))
        .collect::<Vec<_>>()
        .join("\n");

    let contents = format!(
        "pub use sea_orm_migration::prelude::*;\n\n{declarations}\n\n#[derive(DeriveMigrationName)]\npub struct Migrator;\n\n#[async_trait::async_trait]\nimpl MigratorTrait for Migrator {{\n    fn migrations() -> Vec<Box<dyn MigrationTrait>> {{\n        vec![\n{entries}\n        ]\n    }}\n}}\n"
    );
    fs::write(src_dir.join("lib.rs"), contents)
        .map_err(|error| format!("could not update {}: {error}", src_dir.display()))
}

/// Adds the migration crate to the application's `[dependencies]`.
fn add_cargo_dependency(root: &Path, name: &str, spec: &str) -> Result<(), String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("could not read Cargo.toml: {error}"))?;

    if manifest
        .lines()
        .any(|line| line.trim_start().starts_with(name) && line.contains('='))
    {
        return Ok(());
    }

    let updated = insert_dependency(&manifest, name, spec)
        .ok_or_else(|| "could not find a [dependencies] section in Cargo.toml".to_owned())?;
    fs::write(&manifest_path, updated)
        .map_err(|error| format!("could not update Cargo.toml: {error}"))
}

/// Inserts a dependency line into the `[dependencies]` table of a manifest.
fn insert_dependency(manifest: &str, name: &str, spec: &str) -> Option<String> {
    let mut lines: Vec<String> = manifest.lines().map(str::to_owned).collect();
    let section = lines
        .iter()
        .position(|line| line.trim() == "[dependencies]")?;
    let entry = format!("{name} = {{ {spec} }}");

    // Insert after the last entry of the section rather than before the next
    // section header, so the dependency list stays grouped together.
    let mut insert_at = lines.len();
    for (index, line) in lines.iter().enumerate().skip(section + 1) {
        if line.trim_start().starts_with('[') {
            insert_at = index;
            break;
        }
        if !line.trim().is_empty() {
            insert_at = index + 1;
        }
    }

    lines.insert(insert_at, entry);
    // Keep a blank line between the dependency list and the next section.
    if insert_at < lines.len() && lines[insert_at].trim_start().starts_with('[') {
        lines.insert(insert_at, String::new());
    }
    let mut updated = lines.join("\n");
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    Some(updated)
}

/// Adds a crate to the workspace member list.
fn register_workspace_member(root: &Path, member: &str) -> Result<(), String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("could not read Cargo.toml: {error}"))?;

    if manifest.contains(&format!("\"{member}\"")) {
        return Ok(());
    }

    let mut lines: Vec<String> = manifest.lines().map(str::to_owned).collect();
    if !manifest.lines().any(|line| line.trim() == "[workspace]") {
        return Err("could not find a [workspace] section in Cargo.toml".to_owned());
    }
    let members = lines
        .iter()
        .position(|line| line.trim_start().starts_with("members"))
        .ok_or("could not find the workspace members list")?;

    // Normalise a single-line `members = [...]` into a multi-line list we can extend.
    if let Some(start) = lines[members].find('[') {
        let end = lines[members].rfind(']').unwrap_or(start);
        let inner = lines[members][start + 1..end].to_owned();
        let mut items: Vec<String> = inner
            .split(',')
            .map(|item| item.trim().trim_matches('"').to_owned())
            .filter(|item| !item.is_empty())
            .collect();
        items.push(member.to_owned());
        let quoted = items
            .iter()
            .map(|item| format!("\"{item}\""))
            .collect::<Vec<_>>()
            .join(", ");
        lines[members] = format!("members = [{quoted}]");
        let mut updated = lines.join("\n");
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        fs::write(&manifest_path, updated)
            .map_err(|error| format!("could not update Cargo.toml: {error}"))
    } else {
        Err("could not find the workspace members list".to_owned())
    }
}

fn parse_name_and_parent(args: &[String], command: &str) -> Result<(String, Vec<String>), String> {
    let mut name = None;
    let mut parent = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--pmod" {
            index += 1;
            parent = Some(
                args.get(index)
                    .ok_or("`--pmod` requires a parent module path")?
                    .clone(),
            );
        } else if let Some(value) = arg.strip_prefix("--pmod=") {
            parent = Some(value.to_owned());
        } else if arg.starts_with('-') {
            return Err(format!("unknown option `{arg}` for `{command}`"));
        } else if name.is_none() {
            name = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument `{arg}` for `{command}`"));
        }
        index += 1;
    }
    let name = name.ok_or_else(|| format!("missing name. Usage: frs {command} <name>"))?;
    let parents = parent
        .map(|value| {
            value
                .split(['/', '.'])
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok((name, parents))
}

fn add_item(args: &[String]) -> Result<(), String> {
    if args.len() != 2 || args[0] != "entity" {
        return Err("usage: frs add entity <name> (alias for `frs make:entity <name>`)".to_owned());
    }
    make_component(&[args[1].clone()], Component::Entity)
}

fn read_structure(root: &Path) -> Result<Structure, String> {
    let config = fs::read_to_string(root.join("fragment.toml"))
        .map_err(|_| "no fragment.toml found; run `frs create` first".to_owned())?;
    let mut in_structure = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_structure = line == "[structure]";
        } else if in_structure {
            if let Some(value) = line.strip_prefix("type") {
                if let Some(value) = value.split_once('=') {
                    return Structure::parse(value.1.trim().trim_matches('"'));
                }
            }
        }
    }
    Err(
        "fragment.toml is missing [structure].type (expected layered, module, or feature)"
            .to_owned(),
    )
}

fn project_root() -> Result<PathBuf, String> {
    let current =
        env::current_dir().map_err(|error| format!("could not get current directory: {error}"))?;
    if !current.join("Cargo.toml").is_file() {
        return Err(
            "run this command from a Fragment project directory containing Cargo.toml".to_owned(),
        );
    }
    Ok(current)
}

fn write_module_directory(path: &Path, module_name: Option<&str>) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    let mod_file = path.join("mod.rs");
    if !mod_file.exists() {
        write_file(
            &mod_file,
            &format!(
                "//! {} module namespace.\n",
                module_name.unwrap_or("Project")
            ),
        )?;
    }
    Ok(())
}

fn append_module(path: &Path, module: &str) -> Result<(), String> {
    let declaration = format!("pub mod {module};\n");
    let contents = if path.exists() {
        fs::read_to_string(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?
    } else {
        String::new()
    };
    if !contents
        .lines()
        .any(|line| line.trim() == declaration.trim())
    {
        let mut updated = contents;
        updated.push_str(&declaration);
        fs::write(path, updated)
            .map_err(|error| format!("could not update {}: {error}", path.display()))?;
    }
    Ok(())
}

fn run_check(args: &[String]) -> Result<(), String> {
    let path = if args.is_empty() {
        PathBuf::from(".")
    } else if args.len() == 2 && (args[0] == "--path" || args[0] == "-p") {
        PathBuf::from(&args[1])
    } else {
        return Err("usage: frs check [--path <directory>]".to_owned());
    };
    if !path.join("Cargo.toml").is_file() {
        return Err(format!("no Cargo.toml found in {}", path.display()));
    }
    let status = Command::new("cargo")
        .arg("check")
        .current_dir(&path)
        .status()
        .map_err(|error| format!("could not run cargo: {error}"))?;
    if !status.success() {
        return Err(format!("cargo check failed with {status}"));
    }
    Ok(())
}

fn doctor() {
    for tool in ["rustc", "cargo"] {
        match Command::new(tool).arg("--version").output() {
            Ok(output) if output.status.success() => {
                print!("{}: {}", tool, String::from_utf8_lossy(&output.stdout))
            }
            _ => println!("{tool}: not found (install Rust from https://rustup.rs)"),
        }
    }
}

fn validate_package_name(name: &str) -> Result<(), String> {
    validate_identifier(name)?;
    if name
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        return Err("package names must not start with a digit".to_owned());
    }
    Ok(())
}

fn validate_identifier(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return Err("names may contain only ASCII letters, digits, `-`, and `_`".to_owned());
    }
    Ok(())
}

fn to_snake_case(value: &str) -> String {
    let mut result = String::new();
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 && !result.ends_with('_') {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
        } else if character == '-' {
            result.push('_');
        } else {
            result.push(character.to_ascii_lowercase());
        }
    }
    result
}

fn to_pascal_case(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    fs::write(path, contents)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn entity_template(_type_name: &str, table_name: &str) -> String {
    format!("use fragment_rs::orm::entity::prelude::*;\n\n#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n#[sea_orm(table_name = \"{table_name}\")]\npub struct Model {{\n    #[sea_orm(primary_key)]\n    pub id: i32,\n    pub name: String,\n}}\n\n#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\npub enum Relation {{}}\n\nimpl ActiveModelBehavior for ActiveModel {{}}\n")
}

fn migration_manifest(crate_name: &str) -> String {
    let lib_name = crate_name.replace('-', "_");
    format!(
        "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nname = \"{lib_name}\"\npath = \"src/lib.rs\"\n\n[[bin]]\nname = \"{crate_name}\"\npath = \"src/main.rs\"\n\n[dependencies]\nsea-orm-migration = {{ version = \"1.1\", features = [\"cli\", \"runtime-tokio-rustls\", \"sqlx-postgres\", \"sqlx-mysql\", \"sqlx-sqlite\"] }}\nsea-orm = {{ version = \"1.1\", features = [\"runtime-tokio-rustls\", \"sqlx-postgres\", \"sqlx-mysql\", \"sqlx-sqlite\"] }}\ntokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }}\nasync-trait = \"0.1\"\n"
    )
}

fn migration_bin_template(crate_name: &str) -> String {
    let lib_name = crate_name.replace('-', "_");
    format!(
        "use sea_orm_migration::prelude::*;\n\nuse {lib_name}::Migrator;\n\n#[tokio::main]\nasync fn main() {{\n    cli::run_cli(Migrator).await;\n}}\n"
    )
}

fn migration_template(name: &str) -> String {
    format!(
        "use sea_orm_migration::prelude::*;\n\n#[derive(DeriveMigrationName)]\npub struct Migration;\n\n#[async_trait::async_trait]\nimpl MigrationTrait for Migration {{\n    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {{\n        manager\n            .create_table(\n                Table::create()\n                    .table(Tables::Table)\n                    .col(ColumnDef::new(Tables::Id).integer().not_null().auto_increment().primary_key())\n                    .col(ColumnDef::new(Tables::Name).string().not_null())\n                    .to_owned(),\n            )\n            .await\n    }}\n\n    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {{\n        manager\n            .drop_table(Table::drop().table(Tables::Table).to_owned())\n            .await\n    }}\n}}\n\n#[derive(DeriveIden)]\nenum Tables {{\n    Table,\n    Id,\n    Name,\n}}\n\n// Migration: {name}\n"
    )
}

/// The repository the framework lives in.
const FRAMEWORK_REPOSITORY: &str = "https://github.com/Jesse-Dan/fragment-rs";

/// The version of the framework this CLI belongs to.
fn framework_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The version requirement a generated project puts on `fragment-rs`.
///
/// The requirement is the major and minor of this CLI, so a project built by
/// `frs 0.2.1` accepts any `0.2` release.
fn framework_version_requirement() -> String {
    let version = framework_version();
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("{major}.{minor}")
}

/// The release a generated project builds against.
///
/// The tag matches the version of the CLI, so a project built by a released
/// CLI always uses the matching release of the framework.
fn framework_release_tag() -> String {
    format!("v{}", framework_version())
}

/// Where a generated project takes the framework from.
///
/// Once the crate is on crates.io, the project uses the published crate. Until
/// then it takes the framework from the repository, at the release that
/// matches this CLI.
fn framework_dependency() -> String {
    framework_dependency_for(crates_io_has_the_framework())
}

/// The dependency line for the framework.
fn framework_dependency_for(published: bool) -> String {
    if published {
        format!("fragment-rs = \"{}\"", framework_version_requirement())
    } else {
        format!(
            "fragment-rs = {{ version = \"{}\", git = \"{}\", tag = \"{}\" }}",
            framework_version_requirement(),
            FRAMEWORK_REPOSITORY,
            framework_release_tag(),
        )
    }
}

/// Whether crates.io has a release of the framework that a project can use.
///
/// The answer is cached, so a project that makes several files does not ask
/// the registry again.
fn crates_io_has_the_framework() -> bool {
    use std::sync::OnceLock;
    static PUBLISHED: OnceLock<bool> = OnceLock::new();
    *PUBLISHED.get_or_init(|| {
        let url = "https://crates.io/api/v1/crates/fragment-rs";
        let request = ureq::get(url)
            .set(
                "User-Agent",
                "frs-cli (https://github.com/Jesse-Dan/fragment-rs)",
            )
            .timeout(std::time::Duration::from_secs(5));
        match request.call() {
            Ok(response) => response.status() == 200,
            Err(_) => false,
        }
    })
}

fn project_manifest(name: &str) -> String {
    // `axum`, `serde` and `serde_json` are listed explicitly: the `#[controller]`
    // macro expands to `axum::Router` and generated handlers use `axum::Json`,
    // so they have to be direct dependencies of the application crate.
    //
    // The project is a workspace root so that the `migrations` crate created by
    // `frs make:migration` can be registered as a member.
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{}\naxum = \"0.7\"\ntokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n\n[workspace]\nmembers = []\n",
        framework_dependency(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        framework_dependency_for, framework_release_tag, framework_version_requirement,
        generate_migration, make_component, make_migration, make_module, project_manifest,
        scaffold_project, to_pascal_case, to_snake_case, validate_package_name, Component,
        MigrationOptions, Structure,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Commands resolve the project from the working directory, so the tests
    /// that need one take turns.
    fn working_directory_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A unique path for one test.
    ///
    /// The directory is not created: `scaffold_project` refuses to write into
    /// an existing directory.
    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "frs-cli-{label}-{}-{}",
            std::process::id(),
            unique_counter()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    /// A counter that keeps temporary directory names unique within a process.
    fn unique_counter() -> u32 {
        static COUNTER: OnceLock<Mutex<u32>> = OnceLock::new();
        let counter = COUNTER.get_or_init(|| Mutex::new(0));
        let mut value = counter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *value += 1;
        *value
    }

    /// Reads a file, panicking with a useful message when it is missing.
    fn read(path: &Path) -> String {
        std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
    }

    #[test]
    fn name_conversion_is_stable() {
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(
            to_snake_case("authentication-service"),
            "authentication_service"
        );
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
    }

    #[test]
    fn package_names_are_validated() {
        assert!(validate_package_name("2invalid").is_err());
        assert!(validate_package_name("valid-app").is_ok());
    }

    #[test]
    fn module_project_bootstraps_structure_and_config() {
        let path = temp_dir("module");
        scaffold_project(&path, "demo", Structure::Module).unwrap();
        assert!(path
            .join("src/modules/app/controllers/app_controller.rs")
            .exists());
        assert!(read(&path.join("fragment.toml")).contains("type = \"module\""));
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn every_structure_creates_its_directories() {
        let path = temp_dir("structures");
        scaffold_project(&path, "layered", Structure::Layered).unwrap();
        assert!(path.join("src/controllers/mod.rs").exists());
        assert!(path.join("src/repositories/mod.rs").exists());
        assert!(read(&path.join("src/main.rs")).contains("pub mod controllers;"));

        scaffold_project(&path.join("feature"), "feature", Structure::Feature).unwrap();
        assert!(path.join("feature/src/features/app/mod.rs").exists());
        assert!(path.join("feature/src/shared/guards/mod.rs").exists());

        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn generated_manifest_declares_the_framework_dependencies() {
        let path = temp_dir("manifest");
        scaffold_project(&path, "demo", Structure::Module).unwrap();
        let manifest = read(&path.join("Cargo.toml"));
        // The `#[controller]` macro expands to `axum::Router`, so axum has to
        // be a direct dependency of the generated application.
        assert!(manifest.contains("axum = \"0.7\""));
        // The framework is always a direct dependency, from crates.io once it
        // is published and from the repository until then.
        assert!(manifest.contains("fragment-rs ="));
        assert!(manifest.contains("[workspace]"));
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn modules_are_declared_in_the_crate_root_not_a_stray_file() {
        let _guard = working_directory_lock();
        let path = temp_dir("modules");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&path).unwrap();
        let result = make_module(&["authentication".to_owned()]);
        std::env::set_current_dir(previous).unwrap();
        result.unwrap();

        assert!(path
            .join("src/modules/authentication/controllers/authentication_controller.rs")
            .exists());
        assert!(read(&path.join("src/modules/mod.rs")).contains("pub mod authentication;"));
        // The crate root is `src/main.rs`; a stray `src/mod.rs` would shadow it.
        assert!(!path.join("src/mod.rs").exists());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn nested_modules_use_the_parent_path() {
        let _guard = working_directory_lock();
        let path = temp_dir("nested");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&path).unwrap();
        let result = make_module(&["providers".to_owned(), "--pmod=authentication".to_owned()]);
        std::env::set_current_dir(previous).unwrap();
        result.unwrap();

        assert!(path
            .join("src/modules/authentication/providers/controllers/providers_controller.rs")
            .exists());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn entities_add_the_sea_orm_dependency() {
        let _guard = working_directory_lock();
        let path = temp_dir("entities");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&path).unwrap();
        let result = make_component(&["user".to_owned()], Component::Entity);
        std::env::set_current_dir(previous).unwrap();
        result.unwrap();

        // SeaORM's derive macros expand to `sea_orm::` paths, so the entity's
        // crate needs sea-orm as a direct dependency.
        assert!(read(&path.join("Cargo.toml")).contains("sea-orm ="));
        assert!(read(&path.join("src/entities/mod.rs")).contains("pub mod user;"));
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn migrations_create_a_registered_crate() {
        let _guard = working_directory_lock();
        let path = temp_dir("migrations");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&path).unwrap();
        let result = make_migration(&["create_users_table".to_owned()]).await;
        std::env::set_current_dir(previous).unwrap();
        result.unwrap();

        assert!(path.join("migrations/Cargo.toml").is_file());
        assert!(path.join("migrations/src/lib.rs").is_file());
        assert!(path.join("migrations/src/main.rs").is_file());
        assert!(path
            .join("migrations/src/m0001_create_users_table.rs")
            .is_file());

        let manifest = read(&path.join("Cargo.toml"));
        assert!(manifest.contains("\"migrations\""));
        assert!(manifest.contains("demo-migrations = { path = \"migrations\" }"));
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn generated_migration_creates_a_missing_table() {
        let path = temp_dir("generate");
        scaffold_project(&path, "demo", Structure::Module).unwrap();
        std::fs::write(
            path.join("src/entities/user.rs"),
            "use fragment_rs::orm::entity::prelude::*;\n\n#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n#[sea_orm(table_name = \"users\")]\npub struct Model {\n    #[sea_orm(primary_key)]\n    pub id: i32,\n    pub email: String,\n}\n\n#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\npub enum Relation {}\n\nimpl ActiveModelBehavior for ActiveModel {}\n",
        )
        .unwrap();

        let database = path.join("app.db");
        let options = MigrationOptions {
            name: None,
            generate: true,
            database_url: Some(format!("sqlite://{}?mode=rwc", database.display())),
            database: None,
            entities: None,
        };
        generate_migration(&path, &options).await.unwrap();

        let migration = read(&path.join("migrations/src/m0001_create_users.rs"));
        assert!(migration.contains(".create_table("));
        assert!(migration.contains("Users::Email"));
        assert!(migration.contains(".auto_increment().primary_key()"));
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn generated_migration_reports_an_already_matching_database() {
        let path = temp_dir("generate-sync");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        let database = path.join("app.db");
        let url = format!("sqlite://{}?mode=rwc", database.display());

        // Create the table by hand so the database already matches.
        let connection = sea_orm::Database::connect(&url).await.unwrap();
        sea_orm::ConnectionTrait::execute(
            &connection,
            sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Sqlite,
                "CREATE TABLE users (id integer NOT NULL PRIMARY KEY AUTOINCREMENT, email varchar NOT NULL)",
                Vec::new(),
            ),
        )
        .await
        .unwrap();
        connection.close().await.unwrap();

        std::fs::write(
            path.join("src/entities/user.rs"),
            "use fragment_rs::orm::entity::prelude::*;\n\n#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n#[sea_orm(table_name = \"users\")]\npub struct Model {\n    #[sea_orm(primary_key)]\n    pub id: i32,\n    pub email: String,\n}\n\n#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\npub enum Relation {}\n\nimpl ActiveModelBehavior for ActiveModel {}\n",
        )
        .unwrap();

        let options = MigrationOptions {
            name: None,
            generate: true,
            database_url: Some(url),
            database: None,
            entities: None,
        };
        generate_migration(&path, &options).await.unwrap();

        // Nothing changed, so no migration file was written.
        assert!(!path.join("migrations/src/m0001_create_users.rs").exists());
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn generated_migration_adds_a_missing_column_only() {
        let path = temp_dir("generate-column");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        let database = path.join("app.db");
        let url = format!("sqlite://{}?mode=rwc", database.display());

        let connection = sea_orm::Database::connect(&url).await.unwrap();
        sea_orm::ConnectionTrait::execute(
            &connection,
            sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Sqlite,
                "CREATE TABLE users (id integer NOT NULL PRIMARY KEY AUTOINCREMENT, email varchar NOT NULL)",
                Vec::new(),
            ),
        )
        .await
        .unwrap();
        connection.close().await.unwrap();

        std::fs::write(
            path.join("src/entities/user.rs"),
            "use fragment_rs::orm::entity::prelude::*;\n\n#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n#[sea_orm(table_name = \"users\")]\npub struct Model {\n    #[sea_orm(primary_key)]\n    pub id: i32,\n    pub email: String,\n    pub nickname: Option<String>,\n}\n\n#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\npub enum Relation {}\n\nimpl ActiveModelBehavior for ActiveModel {}\n",
        )
        .unwrap();

        let options = MigrationOptions {
            name: None,
            generate: true,
            database_url: Some(url),
            database: None,
            entities: None,
        };
        generate_migration(&path, &options).await.unwrap();

        let migration = read(&path.join("migrations/src/m0001_add_users_nickname.rs"));
        assert!(migration.contains(".add_column("));
        assert!(migration.contains(".null()"));
        // The `up` half never removes anything. The `down` half only undoes
        // what `up` did, so it may drop the column it just added.
        let (up, down) = migration
            .split_once("async fn down")
            .expect("the migration should have an up and a down half");
        assert!(!up.contains("drop_column"));
        assert!(!up.contains("drop_table"));
        assert!(down.contains("drop_column"));
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn the_manifest_points_at_the_framework_release() {
        // Until the crate is on crates.io, the project takes the framework
        // from the repository, at the release that matches this CLI.
        let dependency = framework_dependency_for(false);
        assert!(dependency.contains("git = \"https://github.com/Jesse-Dan/fragment-rs\""));
        assert!(dependency.contains(&format!("tag = \"{}\"", framework_release_tag())));
        assert!(dependency.contains(&format!(
            "version = \"{}\"",
            framework_version_requirement()
        )));
    }

    #[test]
    fn a_published_framework_comes_from_crates_io() {
        // Once the crate is published, the project uses it like any other
        // dependency.
        let dependency = framework_dependency_for(true);
        assert_eq!(
            dependency,
            format!("fragment-rs = \"{}\"", framework_version_requirement())
        );
        assert!(!dependency.contains("git"));
    }

    #[test]
    fn the_manifest_always_depends_on_the_framework() {
        let manifest = project_manifest("demo");
        assert!(manifest.contains("fragment-rs ="));
    }

    #[test]
    fn the_version_requirement_matches_the_cli() {
        let version = env!("CARGO_PKG_VERSION");
        let expected = format!(
            "{}.{}",
            version.split('.').next().unwrap(),
            version.split('.').nth(1).unwrap()
        );
        assert_eq!(framework_version_requirement(), expected);
        assert_eq!(framework_release_tag(), format!("v{version}"));
    }

    #[test]
    fn a_generated_project_reads_its_configuration() {
        let _guard = working_directory_lock();
        let path = temp_dir("bootstrap");
        scaffold_project(&path, "demo", Structure::Module).unwrap();

        // The generated `main.rs` uses the configuration file, so the
        // address in `fragment.toml` is the one the server binds to.
        assert!(read(&path.join("src/main.rs")).contains("from_config_file_or_default"));
        assert!(read(&path.join("fragment.toml")).contains("[server]"));
        let _ = std::fs::remove_dir_all(path);
    }
}
