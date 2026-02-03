interface AppConfig {
  port: number;
  name: string;
}

interface Application {
  start(): void;
  stop(): void;
}

/**
 * Creates a new application instance
 */
export default function createApp(config: AppConfig): Application {
  return {
    start() {
      console.log(`Starting ${config.name} on port ${config.port}`);
    },
    stop() {
      console.log('Stopping application');
    }
  };
}
