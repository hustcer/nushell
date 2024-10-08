use itertools::Itertools;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct PluginList;

impl Command for PluginList {
    fn name(&self) -> &str {
        "plugin list"
    }

    fn signature(&self) -> Signature {
        Signature::build("plugin list")
            .input_output_type(
                Type::Nothing,
                Type::Table(
                    [
                        ("name".into(), Type::String),
                        ("version".into(), Type::String),
                        ("is_running".into(), Type::Bool),
                        ("pid".into(), Type::Int),
                        ("filename".into(), Type::String),
                        ("shell".into(), Type::String),
                        ("commands".into(), Type::List(Type::String.into())),
                    ]
                    .into(),
                ),
            )
            .category(Category::Plugin)
    }

    fn description(&self) -> &str {
        "List installed plugins."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["scope"]
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                example: "plugin list",
                description: "List installed plugins.",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::test_string("inc"),
                    "version" => Value::test_string(env!("CARGO_PKG_VERSION")),
                    "is_running" => Value::test_bool(true),
                    "pid" => Value::test_int(106480),
                    "filename" => if cfg!(windows) {
                        Value::test_string(r"C:\nu\plugins\nu_plugin_inc.exe")
                    } else {
                        Value::test_string("/opt/nu/plugins/nu_plugin_inc")
                    },
                    "shell" => Value::test_nothing(),
                    "commands" => Value::test_list(vec![Value::test_string("inc")]),
                })])),
            },
            Example {
                example: "ps | where pid in (plugin list).pid",
                description: "Get process information for running plugins.",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        // Group plugin decls by plugin identity
        let decls = engine_state.plugin_decls().into_group_map_by(|decl| {
            decl.plugin_identity()
                .expect("plugin decl should have identity")
        });

        // Build plugins list
        let list = engine_state.plugins().iter().map(|plugin| {
            // Find commands that belong to the plugin
            let commands = decls.get(plugin.identity())
                .into_iter()
                .flat_map(|decls| {
                    decls.iter().map(|decl| Value::string(decl.name(), head))
                })
                .collect();

            let pid = plugin
                .pid()
                .map(|p| Value::int(p as i64, head))
                .unwrap_or(Value::nothing(head));

            let shell = plugin
                .identity()
                .shell()
                .map(|s| Value::string(s.to_string_lossy(), head))
                .unwrap_or(Value::nothing(head));

            let metadata = plugin.metadata();
            let version = metadata
                .and_then(|m| m.version)
                .map(|s| Value::string(s, head))
                .unwrap_or(Value::nothing(head));

            let record = record! {
                "name" => Value::string(plugin.identity().name(), head),
                "version" => version,
                "is_running" => Value::bool(plugin.is_running(), head),
                "pid" => pid,
                "filename" => Value::string(plugin.identity().filename().to_string_lossy(), head),
                "shell" => shell,
                "commands" => Value::list(commands, head),
            };

            Value::record(record, head)
        }).collect();

        Ok(Value::list(list, head).into_pipeline_data())
    }
}
