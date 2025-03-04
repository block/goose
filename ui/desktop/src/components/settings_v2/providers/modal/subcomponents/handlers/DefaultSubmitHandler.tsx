export default function DefaultSubmitHandler(configValues) {
  // Log each field value individually for clarity
  console.log('Field values:');
  Object.entries(configValues).forEach(([key, value]) => {
    console.log(`${key}: ${value}`);
  });
}
