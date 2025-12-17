import React from "react";

const RecipeFields = () => {
  return (
    <ul>
      <li><strong>Title</strong> and <strong>description</strong></li>
      <li><strong>Instructions</strong> that tell goose what to do</li>
      <li><strong>Initial prompt</strong> to pre-fill the chat input</li>
      <li><strong>Message</strong> to display at the top of the recipe and <strong>activity buttons</strong> for users to click</li>
      <li><strong>Parameters</strong> to accept dynamic values</li>
      <li><strong>Response JSON schema</strong> for <a href="#structured-output-for-automation">structured output in automations</a></li>
    </ul>
  );
};

export default RecipeFields;
