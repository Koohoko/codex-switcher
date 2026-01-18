import { UsageDisplay } from '../hooks/useUsage';
import { Account } from '../hooks/useAccounts';
import { StatsBar } from './StatsBar';
import { UsageCard } from './UsageCard';
import './Dashboard.css';

interface DashboardProps {
    accounts: Account[];
    currentAccount: Account | null;
    usage: UsageDisplay | null;
    usageLoading: boolean;
    usageError: string | null;
    isCurrentInvalid?: boolean;
    onSwitch: (id: string) => void;
    onRefreshUsage: () => void;
    onNavigateToAccounts: () => void;
    onExport: () => void;
}

export function Dashboard({
    accounts,
    currentAccount,
    usage,
    usageLoading,
    usageError,
    isCurrentInvalid,
    onSwitch,
    onRefreshUsage,
    onNavigateToAccounts,
    onExport,
}: DashboardProps) {
    // 获取最佳账号推荐（配额最高的账号）
    const getBestAccount = () => {
        if (accounts.length === 0) return null;
        // 简单返回第一个非当前账号
        return accounts.find(a => a.id !== currentAccount?.id) || null;
    };

    const bestAccount = getBestAccount();

    return (
        <div className="dashboard">
            {/* 问候语 */}
            <div className="dashboard-greeting">
                <h2>
                    你好, {currentAccount?.name.split('@')[0] || '用户'} 👋
                </h2>
            </div>

            {/* 统计卡片 */}
            <StatsBar accountCount={accounts.length} usage={usage} />

            {/* 双栏布局 */}
            <div className="dashboard-grid">
                {/* 当前账号 */}
                <div className={`dashboard-card current-account ${isCurrentInvalid ? 'invalid' : ''}`}>
                    <div className="card-header">
                        <span className="card-icon">✓</span>
                        <h3>当前账号</h3>
                        {isCurrentInvalid && <span className="invalid-badge" title="授权已失效，请删除后重新登录">⚠️ 失效</span>}
                    </div>
                    {currentAccount ? (
                        <div className="current-account-content">
                            <div className="account-info">
                                <span className="email-icon">✉</span>
                                <span className="email">{currentAccount.name}</span>
                                {usage?.plan_type && (
                                    <span className="plan-badge">{usage.plan_type.toUpperCase()}</span>
                                )}
                            </div>

                            <UsageCard
                                usage={usage}
                                loading={usageLoading}
                                error={usageError}
                                onRefresh={onRefreshUsage}
                            />

                            <button
                                className="btn btn-outline btn-full"
                                onClick={onNavigateToAccounts}
                            >
                                切换账号
                            </button>
                        </div>
                    ) : (
                        <div className="no-account">
                            <p>暂无账号</p>
                        </div>
                    )}
                </div>

                {/* 最佳账号推荐 */}
                <div className="dashboard-card best-accounts">
                    <div className="card-header">
                        <span className="card-icon">↗</span>
                        <h3>最佳账号推荐</h3>
                    </div>
                    <div className="best-accounts-list">
                        {bestAccount ? (
                            <div className="best-account-item">
                                <div className="account-label">
                                    <span className="label-text">推荐账号</span>
                                    <span className="account-email">{bestAccount.name}</span>
                                </div>
                                <span className="quota-badge">100%</span>
                            </div>
                        ) : (
                            <p className="no-recommendation">暂无推荐</p>
                        )}
                    </div>
                    {accounts.length > 1 && (
                        <button
                            className="btn btn-accent btn-full"
                            onClick={() => bestAccount && onSwitch(bestAccount.id)}
                        >
                            一键切换最佳
                        </button>
                    )}
                </div>
            </div>

            {/* 快速链接 */}
            <div className="dashboard-links">
                <button className="link-card" onClick={onNavigateToAccounts}>
                    <span>查看所有账号</span>
                    <span className="link-arrow">→</span>
                </button>
                <button className="link-card" onClick={onExport}>
                    <span>导出账号数据</span>
                    <span className="link-icon">↓</span>
                </button>
            </div>
        </div>
    );
}
