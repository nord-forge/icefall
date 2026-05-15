import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import type { ServerForecast } from '@lib/types';
import { HardDrive, MemoryStick, Cpu, TrendingUp, TrendingDown, Minus, AlertTriangle } from 'lucide-preact';
import styles from './forecast-section.module.css';

type Props = {
  serverId: string;
};

type ResourceCard = {
  label: string;
  icon: typeof HardDrive;
  currentPercent: number;
  dailyRate: number;
  daysUntilFull: number | null;
};

function getTrendIcon(rate: number) {
  if (rate > 0.001) return TrendingUp;
  if (rate < -0.001) return TrendingDown;
  return Minus;
}

function getTrendClass(rate: number): string {
  if (rate > 0.001) return styles.growthUp;
  if (rate < -0.001) return styles.growthDown;
  return '';
}

function formatRate(rate: number): string {
  const abs = Math.abs(rate * 100);
  if (abs < 0.01) return '0%';
  return `${rate > 0 ? '+' : ''}${(rate * 100).toFixed(2)}%`;
}

export default function ForecastSection({ serverId }: Props) {
  const [forecast, setForecast] = useState<ServerForecast | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const { data } = await api.getServerForecast(serverId);
        if (active) setForecast(data);
      } catch (err: any) {
        if (active) setError(err.message || 'Failed to load forecast');
      }
      if (active) setLoading(false);
    }

    load();
    return () => { active = false; };
  }, [serverId]);

  if (loading) {
    return (
      <div class={styles.container}>
        <h2 class={styles.title}>
          <TrendingUp size={18} aria-hidden="true" />
          Resource Forecast
        </h2>
        <p class={styles.infoMessage} role="status" aria-live="polite">
          Loading forecast data...
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div class={styles.container}>
        <h2 class={styles.title}>
          <TrendingUp size={18} aria-hidden="true" />
          Resource Forecast
        </h2>
        <p class={styles.infoMessage} role="status" aria-live="polite">
          {error}
        </p>
      </div>
    );
  }

  if (!forecast || forecast.data_points < 2) {
    return (
      <div class={styles.container}>
        <h2 class={styles.title}>
          <TrendingUp size={18} aria-hidden="true" />
          Resource Forecast
        </h2>
        <p class={styles.infoMessage} role="status" aria-live="polite">
          Not enough data to generate a forecast. Metrics are collected over time to
          identify resource trends.
        </p>
      </div>
    );
  }

  const cards: ResourceCard[] = [
    {
      label: 'Disk',
      icon: HardDrive,
      currentPercent: forecast.disk.current_ratio * 100,
      dailyRate: forecast.disk.daily_growth,
      daysUntilFull: forecast.disk.days_until_full,
    },
    {
      label: 'Memory',
      icon: MemoryStick,
      currentPercent: forecast.memory.current_ratio * 100,
      dailyRate: forecast.memory.daily_growth,
      daysUntilFull: forecast.memory.days_until_full,
    },
    {
      label: 'CPU',
      icon: Cpu,
      currentPercent: forecast.cpu.current_percent,
      dailyRate: forecast.cpu.daily_trend,
      daysUntilFull: null,
    },
  ];

  return (
    <div class={styles.container}>
      <h2 class={styles.title}>
        <TrendingUp size={18} aria-hidden="true" />
        Resource Forecast
      </h2>
      <div class={styles.grid}>
        {cards.map((card) => {
          const TrendIcon = getTrendIcon(card.dailyRate);
          const trendClass = getTrendClass(card.dailyRate);
          const showWarning =
            card.daysUntilFull !== null && card.daysUntilFull < 14;

          return (
            <div key={card.label} class={styles.forecastCard}>
              <div class={styles.cardHeader}>
                <span class={styles.cardTitle}>
                  <card.icon size={16} class={styles.cardIcon} aria-hidden="true" />
                  {card.label}
                </span>
              </div>
              <span class={styles.currentValue}>
                {Math.round(card.currentPercent)}%
              </span>
              <span class={`${styles.growth} ${trendClass}`}>
                <TrendIcon size={14} aria-hidden="true" />
                {formatRate(card.dailyRate)} per day
              </span>
              <span class={styles.daysUntilFull}>
                {card.daysUntilFull !== null
                  ? `${card.daysUntilFull} days until full`
                  : 'Stable'}
              </span>
              {showWarning && (
                <div class={styles.warning} role="alert">
                  <AlertTriangle size={14} aria-hidden="true" />
                  Warning: {card.daysUntilFull} days until full
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
