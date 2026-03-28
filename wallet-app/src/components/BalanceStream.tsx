'use client';
import { useEffect, useState, useRef } from 'react';
import { effectiveBalance, formatUsdcStreaming } from '@/lib/yield';

interface Props {
  baseBalance: number;
  yieldRateBps: number;
  lastUpdate: number;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

export function BalanceStream({ baseBalance, yieldRateBps, lastUpdate, className, size = 'lg' }: Props) {
  const [display, setDisplay] = useState('$0.000000');
  const frameRef = useRef<number>(0);

  useEffect(() => {
    const update = () => {
      const now = Date.now() / 1000;
      const balance = effectiveBalance(baseBalance, yieldRateBps, lastUpdate, now);
      setDisplay(formatUsdcStreaming(balance));
      frameRef.current = requestAnimationFrame(update);
    };
    frameRef.current = requestAnimationFrame(update);
    return () => cancelAnimationFrame(frameRef.current);
  }, [baseBalance, yieldRateBps, lastUpdate]);

  const sizeClass = {
    sm: 'text-lg font-semibold',
    md: 'text-2xl font-bold',
    lg: 'text-4xl font-bold',
  }[size];

  return (
    <span className={`${sizeClass} tabular-nums tracking-tight ${className || ''}`}>
      {display}
    </span>
  );
}
