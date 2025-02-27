// contains basic, common actions like edit, add, delete etc
// logic for whether or not these buttons get shown is stored in the actions/ folder
// specific providers may want specific methods to handle these operations -- TODO
export default interface ConfigurationAction {
  id: string;
  renderButton: () => React.JSX.Element;
}
