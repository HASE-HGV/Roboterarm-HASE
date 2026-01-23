function showStatus(message, isError = false) {
  const banner = document.getElementById('status-banner');
  banner.textContent = message;
  banner.className = 'status-banner visible';

  if (isError) {
    banner.classList.add('error');
  } else {
    banner.classList.add('success');
  }

  setTimeout(() => {
    banner.classList.remove('visible');
    setTimeout(() => {
      banner.className = 'status-banner hidden';
    }, 300);
  }, 5000);
}

async function powerOff() {
  if (!confirm('Are you sure you want to power off the system?')) {
    return;
  }

  try {
    const response = await fetch('/api/poweroff');
    const text = await response.text();
    showStatus(text, !response.ok);
  } catch (error) {
    showStatus(`Error: ${error.message}`, true);
  }
}

async function reboot() {
  if (!confirm('Are you sure you want to reboot the system?')) {
    return;
  }

  try {
    const response = await fetch('/api/reboot');
    const text = await response.text();
    showStatus(text, !response.ok);
  } catch (error) {
    showStatus(`Error: ${error.message}`, true);
  }
}

async function runMotor(motorNumber, clockwise) {
  const steps = parseInt(document.getElementById(`steps-${motorNumber}`).value);
  const stepDelay = document.getElementById(`stepDelay-${motorNumber}`).value;
  const stepStyle = document.getElementById(`stepStyle-${motorNumber}`).value;

  if (isNaN(steps) || steps < 1) {
    showStatus('Please enter a valid number of steps', true);
    return;
  }

  if (isNaN(stepDelay) || stepDelay < 1) {
    showStatus('Please enter a valid step delay', true);
    return;
  }

  const payload = {
    steps: steps,
    clockwise: clockwise,
    stepDelay: stepDelay,
    stepStyle: stepStyle
  };

  try {
    const response = await fetch(`/api/motors/move?motorid=${motorNumber}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    });

    const text = await response.text();
    showStatus(text, !response.ok);
  } catch (error) {
    showStatus(`Error: ${error.message}`, true);
  }
}
