import { Client, Events } from "discord.js";
import { logger } from "../utils/logger";

export default {
  event: Events.Error,
  handler: (client: Client, error: Error) => {
    logger.error("An error occurred:", error);
  },
};
