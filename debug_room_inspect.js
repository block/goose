// Direct Room Inspection Script
// Copy and paste this into your browser console to inspect the specific room

const roomId = "!aiOnzvuIzOOXnluWRv:tchncs.de";

console.log("🔍 DIRECT ROOM INSPECTION for:", roomId);

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.error("❌ matrixService or client not available");
} else {
  const room = matrixService.client.getRoom(roomId);
  if (!room) {
    console.error("❌ Room not found:", roomId);
  } else {
    const timeline = room.getLiveTimeline();
    const events = timeline.getEvents();
    
    console.log("🔍 Room found:", room.name || "Unnamed");
    console.log("🔍 Total events:", events.length);
    
    // Count event types with proper logging
    const eventTypes = events.map(event => event.getType());
    const eventTypeCounts = eventTypes.reduce((acc, type) => {
      acc[type] = (acc[type] || 0) + 1;
      return acc;
    }, {});
    
    console.log("🔍 Event type breakdown:");
    Object.entries(eventTypeCounts).forEach(([type, count]) => {
      console.log(`  ${type}: ${count}`);
    });
    
    // Show first few events in detail
    console.log("🔍 First 5 events in detail:");
    events.slice(0, Math.min(5, events.length)).forEach((event, index) => {
      const content = event.getContent();
      console.log(`📝 Event ${index + 1}:`);
      console.log(`  Type: ${event.getType()}`);
      console.log(`  Sender: ${event.getSender()}`);
      console.log(`  Timestamp: ${new Date(event.getTs()).toISOString()}`);
      console.log(`  Has body: ${!!content.body}`);
      console.log(`  Msgtype: ${content.msgtype || 'none'}`);
      if (content.body) {
        console.log(`  Body preview: ${content.body.substring(0, 200)}...`);
      }
      console.log(`  Full content:`, content);
      console.log("  ---");
    });
    
    // Filter message events specifically
    const messageEvents = events.filter(event => event.getType() === 'm.room.message');
    console.log("🔍 Message events found:", messageEvents.length);
    
    if (messageEvents.length > 0) {
      console.log("🔍 Message events details:");
      messageEvents.forEach((event, index) => {
        const content = event.getContent();
        console.log(`💬 Message ${index + 1}:`);
        console.log(`  Sender: ${event.getSender()}`);
        console.log(`  Timestamp: ${new Date(event.getTs()).toISOString()}`);
        console.log(`  Body: ${content.body || 'No body'}`);
        console.log(`  Msgtype: ${content.msgtype || 'none'}`);
        console.log(`  Full content:`, content);
        console.log("  ---");
      });
    } else {
      console.log("❌ No m.room.message events found!");
      console.log("🔍 Let's check what types of events we DO have:");
      
      // Show all unique event types
      const uniqueTypes = [...new Set(eventTypes)];
      uniqueTypes.forEach(type => {
        const eventsOfType = events.filter(e => e.getType() === type);
        console.log(`📋 ${type} (${eventsOfType.length} events):`);
        
        // Show first event of this type
        if (eventsOfType.length > 0) {
          const firstEvent = eventsOfType[0];
          const content = firstEvent.getContent();
          console.log(`  Sample content:`, content);
          if (content.body) {
            console.log(`  Sample body: ${content.body.substring(0, 100)}...`);
          }
        }
        console.log("  ---");
      });
    }
    
    console.log("🔍 INSPECTION COMPLETE");
  }
}
