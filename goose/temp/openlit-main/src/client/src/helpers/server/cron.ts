import getMessage from "@/constants/messages";
import { Job } from "@/types/cron";
import { consoleLog } from "@/utils/log";
import { execSync } from "child_process";
import { isValidCron } from "cron-validator";
import { existsSync, mkdirSync } from "fs";
import path from "path";

export default class Cron {
	START_MARKER =
		"# Do not modify this section it is being used by OpenLIT ::: Start";
	END_MARKER =
		"# Do not modify this section it is being used by OpenLIT ::: End";

	getCronJobs() {
		try {
			return execSync("crontab -l", { encoding: "utf-8" });
		} catch (error) {
			return ""; // No crontab exists
		}
	}

	validateCronSchedule(schedule: string): void {
		if (!isValidCron(schedule, { alias: true, seconds: false })) {
			throw new Error(getMessage().CRON_RECURRING_TIME_INVALID);
		}
	}

	createLogDirectoryIfNotExists(logPath: string): void {
		const logDir = path.dirname(logPath);
		if (!existsSync(logDir)) {
			mkdirSync(logDir, { recursive: true });
		}
	}

	updateCrontab(job: Job): void {
		try {
			this.validateCronSchedule(job.cronSchedule);
			const currentCrontab = this.getCronJobs();
			const lines = currentCrontab.split("\n");

			const startIdx = lines.indexOf(this.START_MARKER);
			const endIdx = lines.indexOf(this.END_MARKER);

			// Extract existing managed section
			let managedSection: string[] = [];
			if (startIdx !== -1 && endIdx !== -1 && startIdx < endIdx) {
				managedSection = lines.slice(startIdx + 1, endIdx);
			}

			// Remove any existing entry with the same CRON_ID
			managedSection = managedSection.filter(
				(line) => !line.includes(`CRON_ID=${job.cronId}`)
			);

			const envVars = Object.entries(job.cronEnvVars)
				.map(([key, value]) => `${key}=${value}`)
				.join(" ");

			this.createLogDirectoryIfNotExists(job.cronLogPath);

			// Add the new cron job entry
			const newEntry = `${job.cronSchedule} CRON_ID=${job.cronId} ${envVars} $(which node) ${job.cronScriptPath} >> ${job.cronLogPath} 2>&1`;
			managedSection.push(newEntry);

			// Construct the new crontab content
			let newCrontab: string;
			if (startIdx === -1 || endIdx === -1 || startIdx > endIdx) {
				// If section does not exist, create it
				newCrontab = [
					currentCrontab.trim(),
					this.START_MARKER,
					newEntry,
					this.END_MARKER,
				]
					.filter(Boolean)
					.join("\n");
			} else {
				// Replace the existing managed section
				newCrontab = [
					...lines.slice(0, startIdx + 1),
					...managedSection,
					...lines.slice(endIdx),
				].join("\n");
			}

			// Apply the new crontab
			execSync(`echo "${newCrontab}" | crontab -`);
		} catch (error) {
			throw error;
		}
	}

	deleteCronJob(cronId: string): void {
		try {
			const currentCrontab = this.getCronJobs();
			const lines = currentCrontab.split("\n");

			const startIdx = lines.indexOf(this.START_MARKER);
			const endIdx = lines.indexOf(this.END_MARKER);

			if (startIdx === -1 || endIdx === -1 || startIdx > endIdx) {
				consoleLog("Cron job section not found in crontab");
				return;
			}

			const managedSection = lines.slice(startIdx + 1, endIdx);
			const updatedSection = managedSection.filter(
				(line) => !line.includes(`CRON_ID=${cronId}`)
			);

			const newCrontab = [
				...lines.slice(0, startIdx + 1),
				...updatedSection,
				...lines.slice(endIdx),
			].join("\n");

			execSync(`echo "${newCrontab}" | crontab -`);
		} catch (error) {
			throw error;
		}
	}
}
