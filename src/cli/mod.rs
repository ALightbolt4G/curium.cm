use clap::{Arg, Command, ArgAction};

/// Build the CLI definition for the `cm` command.
pub fn build_cli() -> Command {
    Command::new("cm")
        .version("5.0.0")
        .about("Curium compiler and package manager")
        .long_about(
            "The Curium programming language compiler.\n\
            Transpiles .cm source files to C11 and compiles to native binaries."
        )
        .subcommand(
            Command::new("build")
                .about("Compile a Curium source file")
                .arg(
                    Arg::new("file")
                        .help("The .cm source file to compile")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .help("Output file path")
                        .default_value("output"),
                )
                .arg(
                    Arg::new("emit-c")
                        .long("emit-c")
                        .help("Only output the generated C file (do not compile)")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("cc")
                        .long("cc")
                        .help("C compiler to use")
                        .default_value("gcc"),
                ),
        )
        .subcommand(
            Command::new("run")
                .about("Build and execute a Curium program")
                .arg(
                    Arg::new("file")
                        .help("The .cm source file to run")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("check")
                .about("Parse and type-check a Curium file (no codegen)")
                .arg(
                    Arg::new("file")
                        .help("The .cm source file to check")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("dump")
                .about("Debug dump internal representations")
                .subcommand(
                    Command::new("tokens")
                        .about("Print the token stream")
                        .arg(
                            Arg::new("file")
                                .help("The .cm source file")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    Command::new("ast")
                        .about("Print the AST as S-expressions")
                        .arg(
                            Arg::new("file")
                                .help("The .cm source file")
                                .required(true)
                                .index(1),
                        ),
                ),
        )
        .subcommand(
            Command::new("init")
                .about("Initialize a new Curium project")
                .arg(
                    Arg::new("name")
                        .help("Project name")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("fmt")
                .about("Format Curium source files")
                .arg(
                    Arg::new("file")
                        .help("File to format")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("test")
                .about("Run project tests"),
        )
        .subcommand(
            Command::new("doctor")
                .about("Diagnose project health and environment"),
        )
}
