const { createCanvas, loadImage } = require('canvas');
const fs = require('fs');
const path = require('path');

async function generateUpdateIcon() {
  // Load the original icon
  const iconPath = path.join(__dirname, '../src/images/iconTemplate.png');
  const icon = await loadImage(iconPath);
  
  // Create canvas
  const canvas = createCanvas(22, 22);
  const ctx = canvas.getContext('2d');
  
  // Draw the original icon
  ctx.drawImage(icon, 0, 0);
  
  // Add red dot in top-right corner
  ctx.fillStyle = '#FF0000';
  ctx.beginPath();
  ctx.arc(18, 4, 3, 0, 2 * Math.PI);
  ctx.fill();
  
  // Save the new icon
  const outputPath = path.join(__dirname, '../src/images/iconTemplateUpdate.png');
  const buffer = canvas.toBuffer('image/png');
  fs.writeFileSync(outputPath, buffer);
  
  console.log('Generated update icon at:', outputPath);
  
  // Also generate @2x version
  const canvas2x = createCanvas(44, 44);
  const ctx2x = canvas2x.getContext('2d');
  
  // Load and draw @2x version
  const icon2xPath = path.join(__dirname, '../src/images/iconTemplate@2x.png');
  const icon2x = await loadImage(icon2xPath);
  ctx2x.drawImage(icon2x, 0, 0);
  
  // Add red dot in top-right corner (scaled)
  ctx2x.fillStyle = '#FF0000';
  ctx2x.beginPath();
  ctx2x.arc(36, 8, 6, 0, 2 * Math.PI);
  ctx2x.fill();
  
  // Save the @2x version
  const output2xPath = path.join(__dirname, '../src/images/iconTemplateUpdate@2x.png');
  const buffer2x = canvas2x.toBuffer('image/png');
  fs.writeFileSync(output2xPath, buffer2x);
  
  console.log('Generated @2x update icon at:', output2xPath);
}

generateUpdateIcon().catch(console.error);