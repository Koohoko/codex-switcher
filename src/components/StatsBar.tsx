import { UsageDisplay } from '../hooks/useUsage';
import './StatsBar.css';

interface StatsBarProps {
    accountCount: number;
    usage: UsageDisplay | null;
}

export function StatsBar({ accountCount, usage }: StatsBarProps) {
    return (
        <div className="stats-bar">
            <div className="stat-card">
                <div className="stat-icon blue">👤</div>
                <div className="stat-info">
                    <div className="stat-value">{accountCount}</div>
                    <div className="stat-label">账号总数</div>
                </div>
            </div>

            <div className="stat-card">
                <div className="stat-icon green">⏱</div>
                <div className="stat-info">
                    <div className="stat-value">{usage?.five_hour_left ?? '-'}%</div>
                    <div className="stat-label">5h 配额</div>
                    {usage && (
                        <div className={`stat-hint ${usage.five_hour_left > 50 ? 'good' : 'warn'}`}>
                            {usage.five_hour_left > 50 ? '配额充足' : '配额偏低'}
                        </div>
                    )}
                </div>
            </div>

            <div className="stat-card">
                <div className="stat-icon purple">📅</div>
                <div className="stat-info">
                    <div className="stat-value">{usage?.weekly_left ?? '-'}%</div>
                    <div className="stat-label">周配额</div>
                    {usage && (
                        <div className={`stat-hint ${usage.weekly_left > 50 ? 'good' : 'warn'}`}>
                            {usage.weekly_left > 50 ? '配额充足' : '配额偏低'}
                        </div>
                    )}
                </div>
            </div>

            {usage?.has_credits && (
                <div className="stat-card">
                    <div className="stat-icon gold">💰</div>
                    <div className="stat-info">
                        <div className="stat-value">${usage.credits_balance?.toFixed(2) ?? '0.00'}</div>
                        <div className="stat-label">额度余额</div>
                    </div>
                </div>
            )}
        </div>
    );
}
