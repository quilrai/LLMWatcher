import { charts, setCharts, colors, formatNumber, formatLatency, shortenModel } from './utils.js';

// Destroy existing charts
export function destroyCharts() {
  Object.values(charts).forEach(chart => chart.destroy());
  setCharts({});
}

// Create Models Chart (Horizontal Bar)
export function createModelsChart(container, models) {
  const ctx = document.createElement('canvas');
  container.appendChild(ctx);

  const data = models.slice(0, 5); // Top 5 models
  const labels = data.map(m => shortenModel(m.model));
  const values = data.map(m => m.count);

  const newCharts = { ...charts };
  newCharts.models = new Chart(ctx, {
    type: 'bar',
    data: {
      labels,
      datasets: [{
        data: values,
        backgroundColor: [colors.primary, colors.secondary, colors.warning, colors.pink, colors.blue],
        borderRadius: 6,
        barThickness: 24,
      }]
    },
    options: {
      indexAxis: 'y',
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false }
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: { font: { size: 11 } }
        },
        y: {
          grid: { display: false },
          ticks: { font: { size: 11 } }
        }
      }
    }
  });
  setCharts(newCharts);
}

// Create Token Usage Chart (Stacked Bar per request)
export function createTokenChart(container, requests) {
  const ctx = document.createElement('canvas');
  container.appendChild(ctx);

  // Reverse to show oldest first (left to right)
  const data = [...requests].reverse().slice(-15);
  const labels = data.map((_, i) => `#${i + 1}`);

  const newCharts = { ...charts };
  newCharts.tokens = new Chart(ctx, {
    type: 'bar',
    data: {
      labels,
      datasets: [
        {
          label: 'Input',
          data: data.map(r => r.input_tokens),
          backgroundColor: colors.primary,
          borderRadius: 4,
        },
        {
          label: 'Output',
          data: data.map(r => r.output_tokens),
          backgroundColor: colors.secondary,
          borderRadius: 4,
        },
        {
          label: 'Cache Read',
          data: data.map(r => r.cache_read_tokens),
          backgroundColor: colors.warning,
          borderRadius: 4,
        },
      ]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: 'top',
          labels: {
            boxWidth: 12,
            padding: 16,
            font: { size: 11 }
          }
        }
      },
      scales: {
        x: {
          stacked: true,
          grid: { display: false },
          ticks: { font: { size: 10 } }
        },
        y: {
          stacked: true,
          grid: { color: '#f0f0f0' },
          ticks: {
            font: { size: 10 },
            callback: v => formatNumber(v)
          }
        }
      }
    }
  });
  setCharts(newCharts);
}

// Create Latency Chart (Line)
export function createLatencyChart(container, latencyPoints) {
  const ctx = document.createElement('canvas');
  container.appendChild(ctx);

  // Reverse to show oldest first
  const data = [...latencyPoints].reverse();
  const labels = data.map((_, i) => i + 1);
  const values = data.map(p => p.latency_ms);

  const newCharts = { ...charts };
  newCharts.latency = new Chart(ctx, {
    type: 'line',
    data: {
      labels,
      datasets: [{
        label: 'Latency (ms)',
        data: values,
        borderColor: colors.primary,
        backgroundColor: 'rgba(99, 102, 241, 0.1)',
        fill: true,
        tension: 0.3,
        pointRadius: 2,
        pointHoverRadius: 4,
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false }
      },
      scales: {
        x: {
          display: false,
        },
        y: {
          grid: { color: '#f0f0f0' },
          ticks: {
            font: { size: 10 },
            callback: v => formatLatency(v)
          }
        }
      }
    }
  });
  setCharts(newCharts);
}
