export default function OllamaSubmitHandler(configValues: Record<string, unknown>) {
  // Log each field value individually for clarity
  console.log('Ollama field values:');
  Object.entries(configValues).forEach(([key, value]) => {
    console.log(`${key}: ${value}`);
  });
}
