use nodalix_assistant_core::{
    AssistantCore, ContextIndex, IntentRegistry, MockProvider, PermissionBroker,
};

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("help");

    match command {
        "ask" => {
            args.remove(0);
            let query = args.join(" ");
            if query.trim().is_empty() {
                eprintln!("Uso: nodalix-assistant ask <pregunta>");
                std::process::exit(2);
            }
            let core = AssistantCore {
                app_id: "nodalix-assistant".to_string(),
                provider: MockProvider,
                intents: IntentRegistry::load_default(),
                permissions: PermissionBroker::default(),
                context: ContextIndex::default(),
            };
            let response = core.ask(&query);
            println!("{}", response.answer);
            if !response.missing_permissions.is_empty() {
                println!(
                    "Permisos pendientes: {}",
                    response.missing_permissions.join(", ")
                );
            }
            for source in response.sources {
                println!("Fuente: {} · {}", source.connector, source.title);
            }
            for action in response.suggested_actions {
                println!("Acción sugerida: {} ({})", action.title, action.intent_id);
            }
        }
        "grant" => {
            if args.len() != 3 {
                eprintln!("Uso: nodalix-assistant grant <app> <permission>");
                std::process::exit(2);
            }
            let broker = PermissionBroker::default();
            if let Err(err) = broker.grant(&args[1], &args[2]) {
                eprintln!("{err}");
                std::process::exit(1);
            }
            println!("Permiso concedido: {} {}", args[1], args[2]);
        }
        "index-clear" => {
            if let Err(err) = ContextIndex::default().clear() {
                eprintln!("{err}");
                std::process::exit(1);
            }
            println!("Índice local borrado");
        }
        "status" => {
            let registry = IntentRegistry::load_default();
            let index_docs = ContextIndex::default().load_documents().len();
            println!("Nodalix Assistant: fase 0 local-first");
            println!("Intents descubiertos: {}", registry.intents().len());
            println!("Documentos indexados: {index_docs}");
            println!("Provider activo: mock/local");
        }
        _ => {
            println!("Nodalix Assistant");
            println!("Uso:");
            println!("  nodalix-assistant ask <pregunta>");
            println!("  nodalix-assistant status");
            println!("  nodalix-assistant grant <app> <permission>");
            println!("  nodalix-assistant index-clear");
        }
    }
}
