// Import the tracing and tracing-subscriber modules
use tracing::{info, warn, error, span, Level};
use tracing_subscriber::FmtSubscriber;

// The example function we are going to execute
fn expensive_operation() {
    let span = span!(Level::INFO, "expensive_operation");
    let _entered = span.enter();

    info!("Performing an expensive operation");
    // Simulate some time-consuming operations
    std::thread::sleep(std::time::Duration::from_secs(2));
    warn!("The expensive operation took a long time");
}

fn main() {
    // Set up the tracing subscriber
    let subscriber = FmtSubscriber::new();

    // Initialize the tracing subscriber
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    // Execute the example function
    {
        let root_span = span!(Level::INFO, "app");
        let _root_entered = root_span.enter();

        info!("The application starts executing");
        expensive_operation();
        error!("An error has occurred");

        info!("The application execution is complete");  
    }
    {
        let root_span = span!(Level::INFO, "app-2");
        let _root_entered = root_span.enter();
        
        info!("2-The application execution is complete");

        let root_span = span!(Level::INFO, "app-3");
        let _root_entered = root_span.enter();
        
        info!("3-The application execution is complete");
    }
}