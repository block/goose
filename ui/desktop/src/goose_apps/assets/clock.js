class ClockWidget extends GooseWidget {
  constructor(api) {
    super(api);
    this.currentTime = '';
    this.currentDate = '';
    this.timerInterval = null;
  }

  css() {
    return `
            .clock-container {
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                height: 100%;
            }

            .clock-time {
                font-size: 2.5rem;
                font-weight: 700;
                margin-bottom: 10px;
                font-family: monospace;
            }

            .clock-date {
                font-size: 1rem;
                color: #666;
            }

            .clock-settings {
                margin-top: 10px;
                font-size: 0.8rem;
                color: #888;
                cursor: pointer;
            }

            .clock-settings:hover {
                color: #444;
            }
        `;
  }

  onMount() {
    this.updateTime();
    this.timerInterval = setInterval(() => this.updateTime(), 1000);
  }

  onClose() {
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
    }
  }

  bindEvents() {
    this.api.bindEvent('.clock-settings', 'click', () => this.toggleTimeFormat());
  }

  updateTime() {
    const now = new Date();
    const use24Hour = this.api.getProperty('use24Hour', true);

    let hours = now.getHours();
    const minutes = now.getMinutes().toString().padStart(2, '0');
    const seconds = now.getSeconds().toString().padStart(2, '0');

    let timeString;
    if (use24Hour) {
      timeString = `${hours.toString().padStart(2, '0')}:${minutes}:${seconds}`;
    } else {
      const period = hours >= 12 ? 'PM' : 'AM';
      hours = hours % 12;
      hours = hours ? hours : 12; // Convert 0 to 12
      timeString = `${hours}:${minutes}:${seconds} ${period}`;
    }

    const options = { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' };
    const dateString = now.toLocaleDateString(undefined, options);

    this.currentTime = timeString;
    this.currentDate = dateString;
    this.api.update();
  }

  async toggleTimeFormat() {
    const current = this.api.getProperty('use24Hour', true);
    await this.api.setProperty('use24Hour', !current);
    this.updateTime();
  }

  render() {
    return `
            <div class="clock-container">
                <div class="clock-time">${this.currentTime}</div>
                <div class="clock-date">${this.currentDate}</div>
                <div class="clock-settings">Toggle 12/24 Hour</div>
            </div>
        `;
  }

  getDefaultSize() {
    return { width: 300, height: 180 };
  }
}
