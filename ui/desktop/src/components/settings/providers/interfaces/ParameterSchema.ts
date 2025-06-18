export default interface ParameterSchema {
  name: string;
  is_secret: boolean;
  required?: boolean;
  default?: string; // optional default values
}
