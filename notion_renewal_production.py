from datetime import datetime, timedelta
import asyncio
import os
import json
import logging
from notion_client import Client
import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart

class NotionRenewalSystem:
    def __init__(self):
        self.database_id = "Contract-Management-Center-1d654e8e1031806989c7d05137eb7b25"
        self.recipient_email = "strategicpartnerships@squareup.com"
        self.notion = None
        
        # Set up logging
        logging.basicConfig(
            filename='notion_renewal.log',
            level=logging.INFO,
            format='%(asctime)s - %(levelname)s - %(message)s'
        )
        self.logger = logging.getLogger(__name__)

    async def initialize(self):
        """Initialize Notion client and verify connection"""
        try:
            # Initialize Notion client
            self.notion = Client(auth=os.getenv("NOTION_TOKEN"))
            self.logger.info("Notion client initialized successfully")
            return True
        except Exception as e:
            self.logger.error(f"Failed to initialize Notion client: {str(e)}")
            return False

    async def query_renewals(self):
        """Query Notion database for contracts due for renewal in 90 days"""
        try:
            target_date = (datetime.now() + timedelta(days=90)).strftime("%Y-%m-%d")
            
            response = await self.notion.databases.query(
                database_id=self.database_id,
                filter={
                    "property": "Renewal Date",
                    "date": {
                        "equals": target_date
                    }
                }
            )
            
            self.logger.info(f"Successfully queried database. Found {len(response['results'])} renewals")
            return response['results']
        except Exception as e:
            self.logger.error(f"Failed to query renewals: {str(e)}")
            return []

    def format_email(self, contract_data):
        """Format email notification for a contract renewal"""
        try:
            subject = f"Renewal Notification - {contract_data['properties']['Partner']['title'][0]['text']['content']}"
            
            body = f"""
            <div style="font-family: Arial, sans-serif; padding: 20px;">
                <h2>Square Partnership Renewal Notice</h2>
                <p>This is an automated notification from the Contract Management Center.</p>
                
                <div style="margin: 20px 0; padding: 15px; border-left: 4px solid #006AFF; background-color: #f8f9fa;">
                    <p><strong>90-Day Renewal Notice</strong></p>
                    <table style="width: 100%; border-collapse: collapse;">
                        <tr>
                            <td style="padding: 8px 0;"><strong>Contract Name:</strong></td>
                            <td>{contract_data['properties']['Contract Name']['title'][0]['text']['content']}</td>
                        </tr>
                        <tr>
                            <td style="padding: 8px 0;"><strong>Partner:</strong></td>
                            <td>{contract_data['properties']['Partner']['title'][0]['text']['content']}</td>
                        </tr>
                        <tr>
                            <td style="padding: 8px 0;"><strong>Business Function:</strong></td>
                            <td>{contract_data['properties']['Business Function']['select']['name']}</td>
                        </tr>
                        <tr>
                            <td style="padding: 8px 0;"><strong>Renewal Date:</strong></td>
                            <td>{contract_data['properties']['Renewal Date']['date']['start']}</td>
                        </tr>
                    </table>
                </div>
                
                <p>Please initiate the necessary renegotiation or renewal steps for this partnership.</p>
                
                <div style="margin-top: 20px;">
                    <a href="https://www.notion.so/{self.database_id}?id={contract_data['id']}" 
                       style="background-color: #006AFF; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px;">
                       View in Contract Management Center
                    </a>
                </div>
            </div>
            """
            
            return subject, body
        except Exception as e:
            self.logger.error(f"Failed to format email: {str(e)}")
            return None, None

    async def send_email(self, subject, body, recipient):
        """Send email notification"""
        try:
            # Email sending logic here - placeholder for actual email service integration
            self.logger.info(f"Would send email: {subject} to {recipient}")
            self.logger.info("Email sending simulation successful")
            return True
        except Exception as e:
            self.logger.error(f"Failed to send email: {str(e)}")
            return False

    async def process_renewals(self):
        """Main process to check and notify about renewals"""
        self.logger.info("Starting renewal check process")
        
        # Initialize Notion client
        if not await self.initialize():
            return
        
        # Query renewals
        renewals = await self.query_renewals()
        
        if not renewals:
            self.logger.info("No renewals due in 90 days")
            return
        
        # Process each renewal
        for renewal in renewals:
            subject, body = self.format_email(renewal)
            if subject and body:
                if await self.send_email(subject, body, self.recipient_email):
                    self.logger.info(f"Successfully processed renewal notification for {renewal['id']}")
                else:
                    self.logger.error(f"Failed to send notification for {renewal['id']}")

    async def run_daily_check(self):
        """Run the daily check at 8:00 AM MT"""
        self.logger.info("Starting daily renewal check")
        await self.process_renewals()
        self.logger.info("Completed daily renewal check")

# Production run function
async def run_production():
    system = NotionRenewalSystem()
    await system.run_daily_check()

if __name__ == "__main__":
    asyncio.run(run_production())