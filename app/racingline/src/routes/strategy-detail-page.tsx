import { Fragment, useEffect, useRef, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"
import { Link, useNavigate, useParams } from "react-router-dom"
import { createChart, LineSeries, TickMarkType } from "lightweight-charts"
import { ArrowLeft, Trash2 } from "lucide-react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty"
import { Separator } from "@/components/ui/separator"
import { Skeleton } from "@/components/ui/skeleton"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { NavBenchmarkChart } from "@/components/racingline/dashboard/portfolio-overview-board"
import {
  formatMetricValue,
  getMetricToneClassName,
  type Metric,
} from "@/components/racingline/dashboard/portfolio-data"
import {
  useStrategyPortfolioArchiveMutation,
  useStrategyPortfolioNavQuery,
  useStrategyPortfolioPerformanceQuery,
  useStrategyPortfolioPositionsQuery,
  useStrategyPortfolioQuery,
  useStrategyPortfolioRebalanceRecordsQuery,
  useStrategyPortfolioStatementQuery,
  useStrategyPortfolioSignalsQuery,
  useStrategyPortfolioSignalTimelineQuery,
  useStrategyPortfolioVirtualAccountQuery,
} from "@/api/hooks"
import { queryKeys } from "@/api/queryKeys"
import { ApiError } from "@/api/client"
import { cn } from "@/lib/utils"
import { Spinner } from "@/components/ui/spinner"
import {
  isArchivedPortfolioError,
  strategyPortfolioArchiveErrorMessage,
} from "@/routes/strategy-detail-utils"
import type {
  StrategyPortfolioDashboardCard,
  StrategyBacktestNavPoint,
  StrategyBacktestRebalanceRecord,
  StrategyBacktestTargetRecord,
  StrategyPortfolioPerformanceView,
  StrategyPortfolioRecord,
  StrategyPortfolioSignalTimelinePoint,
  StrategyPortfolioStatementPeriodKey,
  StrategyPortfolioStatementResponse,
  StrategyPortfolioVirtualAccount,
} from "@/types/rearview"

type DetailTradeDirection = "buy" | "hold" | "sell"

type DetailTrade = {
  changePercent: string
  contribution: string
  costPrice: string
  currentPrice: string
  direction: DetailTradeDirection
  holdingDays: string
  rebalanceReason: string
  securityCode: string
  securityName: string
}

type DetailRebalanceRecord = {
  date: string
  trades: DetailTrade[]
}

type SignalScoreItem = {
  label: string
  score: number
}

type SignalStock = {
  code: string
  executionDate?: string
  name: string
  rank?: number
  score: number
  scoreItems: SignalScoreItem[]
  signalDate?: string
}

type SignalPool = {
  date: string
  signalCount: number
  stocks: SignalStock[]
}

const STATEMENT_PAGE_LIMIT = 100

const STATEMENT_PERIOD_OPTIONS: {
  key: StrategyPortfolioStatementPeriodKey
  label: string
}[] = [
  { key: "month", label: "本月" },
  { key: "three_months", label: "近三月" },
  { key: "six_months", label: "近半年" },
  { key: "ytd", label: "今年" },
  { key: "all", label: "全部" },
]

function StrategyDetailPage() {
  const { portfolioId } = useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [selectedSignalDate, setSelectedSignalDate] = useState("")
  const [selectedRebalanceDate, setSelectedRebalanceDate] = useState("")
  const [statementPeriod, setStatementPeriod] =
    useState<StrategyPortfolioStatementPeriodKey>("month")
  const [statementOffset, setStatementOffset] = useState(0)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [deleteError, setDeleteError] = useState<string | null>(null)
  const rebalanceDateScrollerRef = useRef<HTMLDivElement | null>(null)
  const strategyPortfolioId = portfolioId ?? null
  const portfolioQuery = useStrategyPortfolioQuery(strategyPortfolioId)
  const portfolioRecord = portfolioQuery.data
  const archiveMutation = useStrategyPortfolioArchiveMutation()
  const archivedPortfolioError = isArchivedPortfolioError(portfolioQuery.error)
  const isKnownPendingFirstRun =
    portfolioRecord?.live_status === "pending_first_run"
  const liveResultPortfolioId =
    !portfolioRecord || isKnownPendingFirstRun ? null : strategyPortfolioId
  const navQuery = useStrategyPortfolioNavQuery(liveResultPortfolioId)
  const performanceQuery = useStrategyPortfolioPerformanceQuery(
    liveResultPortfolioId
  )
  const virtualAccountQuery = useStrategyPortfolioVirtualAccountQuery(
    liveResultPortfolioId
  )
  const statementQuery = useStrategyPortfolioStatementQuery(
    liveResultPortfolioId,
    {
      period: statementPeriod,
      limit: STATEMENT_PAGE_LIMIT,
      offset: statementOffset,
    }
  )
  const signalTimelineQuery =
    useStrategyPortfolioSignalTimelineQuery(strategyPortfolioId)
  const latestNavDate = navQuery.data?.points.at(-1)?.trade_date ?? null
  const latestSignalDate =
    selectedSignalDate ||
    latestNavDate ||
    signalTimelineQuery.data?.trade_dates.at(-1)?.trade_date ||
    null
  const signalsQuery = useStrategyPortfolioSignalsQuery(
    strategyPortfolioId,
    latestSignalDate
  )
  const positionsQuery = useStrategyPortfolioPositionsQuery(
    liveResultPortfolioId,
    latestNavDate
  )
  const rebalanceRecordsQuery = useStrategyPortfolioRebalanceRecordsQuery(
    liveResultPortfolioId,
    selectedRebalanceDate || null
  )
  const isPendingFirstRun =
    isKnownPendingFirstRun ||
    isPortfolioPendingFirstRunError(navQuery.error) ||
    isPortfolioPendingFirstRunError(performanceQuery.error) ||
    isPortfolioPendingFirstRunError(virtualAccountQuery.error) ||
    isPortfolioPendingFirstRunError(statementQuery.error) ||
    isPortfolioPendingFirstRunError(positionsQuery.error) ||
    isPortfolioPendingFirstRunError(rebalanceRecordsQuery.error)
  const portfolio = portfolioRecord
    ? buildDetailPortfolioView(
        portfolioRecord,
        isPendingFirstRun ? [] : (navQuery.data?.points ?? []),
        isPendingFirstRun ? null : (performanceQuery.data ?? null)
      )
    : null
  const signalPools = buildSignalPoolsFromApi(
    signalTimelineQuery.data?.trade_dates ?? [],
    latestSignalDate,
    signalsQuery.data?.items ?? [],
    signalsQuery.data?.pending_buy_signals ?? []
  )
  const records = isPendingFirstRun
    ? []
    : (rebalanceRecordsQuery.data?.records ?? []).map(mapApiRebalanceRecord)
  const livePositionCount =
    isPendingFirstRun || positionsQuery.isError
      ? null
      : (positionsQuery.data?.items.length ?? null)

  useEffect(() => {
    const scroller = rebalanceDateScrollerRef.current

    if (!scroller) {
      return
    }

    scroller.scrollLeft = scroller.scrollWidth
  }, [records.length])

  useEffect(() => {
    if (archivedPortfolioError) {
      navigate("/dashboard", { replace: true, viewTransition: true })
    }
  }, [archivedPortfolioError, navigate])

  async function handleDeleteConfirm() {
    if (!strategyPortfolioId) {
      return
    }

    setDeleteError(null)

    try {
      await archiveMutation.mutateAsync(strategyPortfolioId)
      await queryClient.invalidateQueries({
        queryKey: queryKeys.strategyPortfolioDashboard(),
      })
      queryClient.removeQueries({
        queryKey: queryKeys.strategyPortfolio(strategyPortfolioId),
      })
      setDeleteDialogOpen(false)
      navigate("/dashboard", { viewTransition: true })
    } catch (error) {
      setDeleteError(strategyPortfolioArchiveErrorMessage(error))
    }
  }

  if (portfolioQuery.isLoading) {
    return (
      <Empty className="min-h-[calc(100svh-8rem)] border border-dashed border-border/70">
        <EmptyHeader>
          <EmptyTitle>策略加载中</EmptyTitle>
          <EmptyDescription>正在从 Rearview 读取策略组合。</EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  if (archivedPortfolioError) {
    return (
      <Empty className="min-h-[calc(100svh-8rem)] border border-dashed border-border/70">
        <EmptyHeader>
          <EmptyTitle>策略已删除</EmptyTitle>
          <EmptyDescription>正在返回策略看板。</EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  if (portfolioQuery.isError || !portfolio || !portfolioRecord) {
    return (
      <Empty className="min-h-[calc(100svh-8rem)] border border-dashed border-border/70">
        <EmptyHeader>
          <EmptyTitle>未找到策略</EmptyTitle>
          <EmptyDescription>策略不存在或链接无效。</EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button
            render={<Link to="/dashboard" viewTransition />}
            nativeButton={false}
            size="lg"
          >
            返回策略看板
          </Button>
        </EmptyContent>
      </Empty>
    )
  }

  const selectedRebalanceRecord =
    records.find((record) => record.date === selectedRebalanceDate) ??
    records.at(-1)
  const selectedSignalPool =
    signalPools.find((pool) => pool.date === selectedSignalDate) ??
    signalPools.at(-1)
  const selectedRebalanceTradeSections = selectedRebalanceRecord
    ? buildRebalanceTradeSections(selectedRebalanceRecord.trades)
    : []
  const latestPoint = portfolio.curve.at(-1)
  const previousPoint = portfolio.curve.at(-2)
  const latestStrategyReturn =
    latestPoint && previousPoint
      ? latestPoint.nav / previousPoint.nav - 1
      : null
  const latestBenchmarkReturn =
    latestPoint && previousPoint
      ? latestPoint.benchmark / previousPoint.benchmark - 1
      : null
  const latestExcessReturn = latestPoint
    ? latestPoint.nav - latestPoint.benchmark
    : null
  const returnMetrics = buildReturnMetrics(
    portfolio.returns,
    latestExcessReturn,
    portfolio.dailyWinRate
  )
  const metricGroups = [
    { title: "收益指标", metrics: returnMetrics },
    { title: "风险指标", metrics: portfolio.risk },
    { title: "性价比", metrics: portfolio.efficiency },
    { title: "相对市场", metrics: portfolio.relative },
  ]

  return (
    <section className="mx-auto flex min-h-[calc(100svh-8rem)] w-full max-w-[72rem] flex-col gap-4">
      <div className="flex h-9 min-w-0 items-center gap-3">
        <Button
          render={<Link to="/dashboard" viewTransition />}
          nativeButton={false}
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground hover:bg-muted/60 hover:text-foreground"
          aria-label="返回策略看板"
        >
          <ArrowLeft />
        </Button>
        <h1 className="truncate text-lg font-medium">{portfolio.name}</h1>
        <div aria-hidden="true" className="h-5 w-px shrink-0 bg-border/90" />
        <div className="flex min-w-0 flex-1 items-center gap-3 overflow-hidden text-xs text-muted-foreground tabular-nums">
          <span className="shrink-0">建仓: {portfolio.startDate}</span>
          <span className="shrink-0">运行: {portfolio.simulationDays} 天</span>
          {livePositionCount !== null ? (
            <span className="shrink-0">持仓: {livePositionCount} 只</span>
          ) : null}
        </div>
        <Dialog
          open={deleteDialogOpen}
          onOpenChange={(open) => {
            if (archiveMutation.isPending) {
              return
            }
            setDeleteDialogOpen(open)
            if (open) {
              setDeleteError(null)
            }
          }}
        >
          <DialogTrigger
            render={
              <Button
                disabled={archiveMutation.isPending}
                variant="ghost"
                size="sm"
                type="button"
                className="shrink-0 text-muted-foreground hover:bg-muted/60 hover:text-destructive"
              />
            }
          >
            <Trash2 data-icon="inline-start" />
            删除
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除策略</DialogTitle>
              <DialogDescription>
                删除后该策略将从看板移除，历史回测和运行记录会保留。
              </DialogDescription>
            </DialogHeader>
            {deleteError ? (
              <Alert variant="destructive">
                <AlertTitle>删除失败</AlertTitle>
                <AlertDescription>{deleteError}</AlertDescription>
              </Alert>
            ) : null}
            <DialogFooter>
              <DialogClose
                render={
                  <Button
                    disabled={archiveMutation.isPending}
                    variant="outline"
                    type="button"
                  />
                }
              >
                取消
              </DialogClose>
              <Button
                disabled={archiveMutation.isPending}
                onClick={handleDeleteConfirm}
                type="button"
                variant="destructive"
              >
                {archiveMutation.isPending ? (
                  <Spinner data-icon="inline-start" />
                ) : null}
                {archiveMutation.isPending ? "删除中" : "确认删除"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      <div className="flex min-h-full flex-col gap-4 pt-2">
        <div className="flex min-h-0 flex-col gap-4">
          <div className="flex w-full flex-col gap-4">
            <section className="flex flex-col gap-3">
              <div className="flex items-center justify-between gap-3">
                <div className="text-sm font-medium">
                  {isPendingFirstRun ? "待调入信号" : "策略信号"}
                </div>
                {selectedSignalPool ? (
                  <div className="text-xs text-muted-foreground tabular-nums">
                    {selectedSignalPool.date} / {selectedSignalPool.signalCount}{" "}
                    只
                  </div>
                ) : null}
              </div>

              <div className="grid gap-4 xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
                <div className="flex min-h-0 flex-col gap-2">
                  <div className="text-xs font-medium text-muted-foreground">
                    历史信号数
                  </div>
                  <SignalCountChart
                    onTimeSelect={setSelectedSignalDate}
                    points={signalPools.map((pool) => ({
                      time: pool.date,
                      value: pool.signalCount,
                    }))}
                    selectedTime={selectedSignalPool?.date}
                  />
                </div>

                {selectedSignalPool ? (
                  <div className="flex min-h-0 flex-col gap-2">
                    <div className="grid h-[18px] shrink-0 gap-1 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
                      <div className="min-w-0">
                        <div className="text-sm font-medium tabular-nums">
                          {selectedSignalPool.date}
                        </div>
                      </div>
                      <div className="text-xs text-muted-foreground tabular-nums">
                        {selectedSignalPool.stocks.length} 只
                      </div>
                    </div>

                    <div className="h-[14rem] min-h-0 overflow-y-auto">
                      <Table className="w-full table-fixed">
                        <TableHeader>
                          <TableRow className="hover:bg-transparent">
                            <TableHead className="h-7 w-[9.5rem] px-1">
                              股票
                            </TableHead>
                            <TableHead className="h-7 px-1">
                              {isPendingFirstRun ? "信号 / 建仓" : "得分项"}
                            </TableHead>
                            <TableHead className="h-7 w-16 px-1 text-right">
                              得分
                            </TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {selectedSignalPool.stocks.map((stock) => (
                            <TableRow key={stock.code}>
                              <TableCell className="px-1 py-1">
                                <div className="grid min-w-0 grid-cols-[4.5em_minmax(0,1fr)] items-center gap-1">
                                  <span className="truncate font-medium">
                                    {stock.name}
                                  </span>
                                  <span className="truncate text-muted-foreground tabular-nums">
                                    {stock.code}
                                  </span>
                                </div>
                              </TableCell>
                              <TableCell className="px-1 py-1">
                                <div
                                  className="w-full truncate text-muted-foreground tabular-nums"
                                  title={formatSignalDetail(stock)}
                                >
                                  {formatSignalDetail(stock)}
                                </div>
                              </TableCell>
                              <TableCell className="px-1 py-1 text-right">
                                <Badge
                                  variant={getScoreBadgeVariant(stock.score)}
                                >
                                  {stock.score.toFixed(1)}
                                </Badge>
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </div>
                  </div>
                ) : null}
              </div>
            </section>

            <Separator className="bg-border/60" />

            <section className="flex flex-col gap-3">
              <div className="flex items-baseline gap-2">
                <div className="text-sm font-medium">策略业绩</div>
              </div>

              <div className="flex flex-col gap-4">
                <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                  <DetailSummaryMetric
                    label="业绩日期"
                    value={
                      isPendingFirstRun
                        ? "待建仓"
                        : (latestPoint?.time ?? portfolio.startDate)
                    }
                  />
                  <DetailSummaryMetric
                    label="业绩基准"
                    value="沪深300"
                    detail="000300.SH"
                  />
                  {latestPoint ? (
                    <>
                      <DetailSummaryMetric
                        label="策略净值"
                        value={latestPoint.nav.toFixed(4)}
                        detail={formatOptionalSignedPercent(
                          latestStrategyReturn
                        )}
                        detailClassName={getSignedValueClassName(
                          formatOptionalSignedPercent(latestStrategyReturn)
                        )}
                      />
                      <DetailSummaryMetric
                        label="基准净值"
                        value={latestPoint.benchmark.toFixed(4)}
                        detail={formatOptionalSignedPercent(
                          latestBenchmarkReturn
                        )}
                        detailClassName={getSignedValueClassName(
                          formatOptionalSignedPercent(latestBenchmarkReturn)
                        )}
                      />
                    </>
                  ) : null}
                </div>
                {isPendingFirstRun ? (
                  <Empty className="border border-dashed border-border/70 py-8">
                    <EmptyHeader>
                      <EmptyTitle>尚未产生 live 业绩</EmptyTitle>
                      <EmptyDescription>
                        首个建仓日运行成功后，这里会展示组合 live 净值和绩效。
                      </EmptyDescription>
                    </EmptyHeader>
                  </Empty>
                ) : (
                  <div className="grid gap-x-6 gap-y-3 md:grid-cols-2 xl:grid-cols-4">
                    {metricGroups.map((group) => (
                      <MetricGroup
                        key={group.title}
                        title={group.title}
                        metrics={group.metrics}
                      />
                    ))}
                  </div>
                )}
              </div>
            </section>

            <Separator className="bg-border/60" />

            <section className="flex flex-col gap-3">
              <div className="flex items-center justify-between gap-3">
                <div className="text-sm font-medium">净值走势</div>
                <div className="flex items-center gap-4 text-xs text-muted-foreground">
                  <div className="inline-flex items-center gap-1.5">
                    <span className="size-2 bg-foreground" />
                    策略净值
                  </div>
                  <div className="inline-flex items-center gap-1.5">
                    <span className="size-2 bg-muted-foreground" />
                    基准净值
                  </div>
                </div>
              </div>
              {portfolio.curve.length > 0 ? (
                <NavBenchmarkChart
                  className="h-60"
                  height={240}
                  points={portfolio.curve}
                />
              ) : (
                <Empty className="h-60 border">
                  <EmptyHeader>
                    <EmptyTitle>暂无净值数据</EmptyTitle>
                    <EmptyDescription>
                      {isPendingFirstRun
                        ? "当前组合待建仓，尚未产生 live 净值曲线。"
                        : "当前组合尚未产生可展示的净值曲线。"}
                    </EmptyDescription>
                  </EmptyHeader>
                </Empty>
              )}
            </section>

            <Separator className="bg-border/60" />

            <section className="flex flex-col gap-3">
              <div className="flex items-center justify-between gap-3">
                <div className="text-sm font-medium">持仓记录</div>
                <div className="text-xs text-muted-foreground tabular-nums">
                  {records.length} 个调仓日
                </div>
              </div>

              <div
                ref={rebalanceDateScrollerRef}
                className="h-[32px] shrink-0 [scrollbar-width:thin] overflow-x-auto overflow-y-hidden overscroll-x-contain pb-3 [&::-webkit-scrollbar]:h-[2px] [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent"
              >
                <div className="flex min-w-max gap-1.5 pr-1">
                  {records.map((record) => {
                    const isSelected =
                      record.date === selectedRebalanceRecord?.date
                    const positionCount = getPositionCount(record.trades)

                    return (
                      <Button
                        key={record.date}
                        aria-label={`${record.date} 持仓 ${positionCount} 只`}
                        aria-pressed={isSelected}
                        data-state={isSelected ? "selected" : "idle"}
                        className="grid h-[18px] w-[6.25rem] shrink-0 grid-cols-[2.75rem_2rem] items-center justify-center gap-1 px-1 text-muted-foreground hover:bg-muted data-[state=selected]:bg-muted data-[state=selected]:text-foreground"
                        size="sm"
                        type="button"
                        variant="ghost"
                        onClick={() => setSelectedRebalanceDate(record.date)}
                      >
                        <span className="text-right text-[11px] leading-none tabular-nums">
                          {formatCompactDate(record.date)}
                        </span>
                        <span className="text-left text-[11px] leading-none font-normal tabular-nums opacity-80">
                          {positionCount}只
                        </span>
                      </Button>
                    )
                  })}
                </div>
              </div>

              {selectedRebalanceRecord ? (
                <div className="flex flex-col gap-2">
                  <div className="flex items-end justify-between gap-3">
                    <div className="text-sm font-medium tabular-nums">
                      {selectedRebalanceRecord.date}
                    </div>
                    <div className="text-xs text-muted-foreground tabular-nums">
                      调入{" "}
                      {
                        selectedRebalanceRecord.trades.filter(
                          (trade) => trade.direction === "buy"
                        ).length
                      }{" "}
                      只 / 持有{" "}
                      {
                        selectedRebalanceRecord.trades.filter(
                          (trade) => trade.direction === "hold"
                        ).length
                      }{" "}
                      只 / 调出{" "}
                      {
                        selectedRebalanceRecord.trades.filter(
                          (trade) => trade.direction === "sell"
                        ).length
                      }{" "}
                      只
                    </div>
                  </div>

                  <Table className="w-full table-fixed text-xs leading-snug [&_td]:overflow-hidden [&_th]:overflow-hidden">
                    <TableHeader>
                      <TableRow className="hover:bg-transparent">
                        <TableHead className="h-6 w-[17%] px-1">股票</TableHead>
                        <TableHead className="h-6 w-[17%] px-1">
                          调仓理由
                        </TableHead>
                        <TableHead className="h-6 w-[11%] px-1 text-right">
                          持仓天数
                        </TableHead>
                        <TableHead className="h-6 w-[12%] px-1 text-right">
                          涨跌幅
                        </TableHead>
                        <TableHead className="h-6 w-[14%] px-1 text-right">
                          成本价
                        </TableHead>
                        <TableHead className="h-6 w-[14%] px-1 text-right">
                          现价
                        </TableHead>
                        <TableHead className="h-6 w-[15%] px-1 text-right">
                          收益贡献
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {selectedRebalanceTradeSections.map((section) => (
                        <Fragment key={section.direction}>
                          <TableRow className="bg-muted/30 hover:bg-muted/30">
                            <TableCell
                              colSpan={7}
                              className="px-1 py-1 text-xs font-medium text-muted-foreground"
                            >
                              {formatTradeDirection(section.direction)}{" "}
                              {section.trades.length} 只
                            </TableCell>
                          </TableRow>
                          {section.trades.map((trade, tradeIndex) => (
                            <TableRow
                              key={`${selectedRebalanceRecord.date}-${trade.direction}-${trade.securityCode}-${tradeIndex}`}
                            >
                              <TableCell className="px-1 py-0.5">
                                <div className="grid min-w-0 grid-cols-[4.25em_minmax(0,1fr)] items-center gap-1">
                                  <span className="truncate font-medium">
                                    {trade.securityName}
                                  </span>
                                  <span className="truncate text-muted-foreground tabular-nums">
                                    {trade.securityCode}
                                  </span>
                                </div>
                              </TableCell>
                              <TableCell className="px-1 py-0.5">
                                <span className="block truncate text-muted-foreground">
                                  {trade.rebalanceReason || "-"}
                                </span>
                              </TableCell>
                              <TableCell className="px-1 py-0.5 text-right tabular-nums">
                                {trade.holdingDays}
                              </TableCell>
                              <TableCell
                                className={cn(
                                  "px-1 py-0.5 text-right font-medium tabular-nums",
                                  getSignedValueClassName(trade.changePercent)
                                )}
                              >
                                {trade.changePercent}
                              </TableCell>
                              <TableCell className="px-1 py-0.5 text-right tabular-nums">
                                {trade.costPrice}
                              </TableCell>
                              <TableCell className="px-1 py-0.5 text-right tabular-nums">
                                {trade.currentPrice}
                              </TableCell>
                              <TableCell
                                className={cn(
                                  "px-1 py-0.5 text-right font-medium tabular-nums",
                                  getSignedValueClassName(trade.contribution)
                                )}
                              >
                                {trade.contribution}
                              </TableCell>
                            </TableRow>
                          ))}
                        </Fragment>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              ) : (
                <Empty className="border border-dashed border-border/70 py-8">
                  <EmptyHeader>
                    <EmptyTitle>暂无持仓记录</EmptyTitle>
                    <EmptyDescription>
                      {isPendingFirstRun
                        ? "当前组合待建仓，首个 live daily run 成功后会生成持仓记录。"
                        : "当前组合尚未产生可展示的调仓记录。"}
                    </EmptyDescription>
                  </EmptyHeader>
                </Empty>
              )}
            </section>

            <Separator className="bg-border/60" />

            <VirtualAccountSection
              account={virtualAccountQuery.data ?? null}
              initialCash={
                portfolioRecord.execution_config.account.initial_cash
              }
              isError={
                !isPendingFirstRun &&
                virtualAccountQuery.isError &&
                !isPortfolioPendingFirstRunError(virtualAccountQuery.error)
              }
              isLoading={!isPendingFirstRun && virtualAccountQuery.isLoading}
              isPendingFirstRun={isPendingFirstRun}
            />

            <Separator className="bg-border/60" />

            <StatementSection
              statement={statementQuery.data ?? null}
              period={statementPeriod}
              offset={statementOffset}
              isError={
                !isPendingFirstRun &&
                statementQuery.isError &&
                !isPortfolioPendingFirstRunError(statementQuery.error)
              }
              isLoading={!isPendingFirstRun && statementQuery.isLoading}
              isPendingFirstRun={isPendingFirstRun}
              onNextPage={() =>
                setStatementOffset((current) => current + STATEMENT_PAGE_LIMIT)
              }
              onPeriodChange={(period) => {
                setStatementPeriod(period)
                setStatementOffset(0)
              }}
              onPreviousPage={() =>
                setStatementOffset((current) =>
                  Math.max(0, current - STATEMENT_PAGE_LIMIT)
                )
              }
            />
          </div>
        </div>
      </div>
    </section>
  )
}

function DetailSummaryMetric({
  detail,
  detailClassName,
  label,
  value,
  valueClassName,
}: {
  detail?: string
  detailClassName?: string
  label: string
  value: string
  valueClassName?: string
}) {
  return (
    <div className="min-w-0">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div
        className={cn(
          "mt-1 flex min-w-0 items-baseline gap-2 truncate text-sm font-medium tabular-nums"
        )}
      >
        <span className={cn("truncate", valueClassName)}>{value}</span>
        {detail ? (
          <span
            className={cn(
              "truncate text-xs font-normal text-muted-foreground tabular-nums",
              detailClassName
            )}
          >
            {detail}
          </span>
        ) : null}
      </div>
    </div>
  )
}

function MetricGroup({ metrics, title }: { metrics: Metric[]; title: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <div className="text-xs font-medium text-muted-foreground">{title}</div>
      <div className="flex flex-col gap-1.5">
        {metrics.map((metric) => (
          <div
            key={metric.label}
            className="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3"
          >
            <div className="truncate text-xs text-muted-foreground">
              {metric.label}
            </div>
            <div
              className={cn(
                "text-sm font-medium tabular-nums",
                getMetricToneClassName(metric)
              )}
            >
              {formatMetricValue(metric)}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

function VirtualAccountSection({
  account,
  initialCash,
  isError,
  isLoading,
  isPendingFirstRun,
}: {
  account: StrategyPortfolioVirtualAccount | null
  initialCash: number
  isError: boolean
  isLoading: boolean
  isPendingFirstRun: boolean
}) {
  const inceptionPnl = account ? account.total_equity - initialCash : null
  const metrics = account
    ? [
        {
          label: "账户资产",
          tone: "neutral" as const,
          value: formatAccountCurrency(account.total_equity),
        },
        {
          label: "持股市值",
          tone: "neutral" as const,
          value: formatAccountCurrency(account.position_market_value),
        },
        {
          label: "可用金额",
          tone: "neutral" as const,
          value: formatAccountCurrency(account.cash_balance),
        },
        {
          label: "总盈亏",
          tone: signedTone(inceptionPnl),
          value: formatAccountOptionalSignedCurrency(inceptionPnl),
        },
        {
          label: "当日盈亏",
          tone: signedTone(account.daily_pnl),
          value: formatAccountOptionalSignedCurrency(account.daily_pnl),
        },
        {
          label: "当日盈亏比",
          tone: signedTone(account.daily_return),
          value: formatAccountOptionalSignedPercent(account.daily_return),
        },
      ]
    : []

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="text-sm font-medium">虚拟资金账户</div>
        {account ? (
          <div className="text-xs text-muted-foreground tabular-nums">
            {account.account_date}
          </div>
        ) : null}
      </div>

      {isPendingFirstRun ? (
        <Empty className="py-8">
          <EmptyHeader>
            <EmptyTitle>尚未产生虚拟资金账户</EmptyTitle>
            <EmptyDescription>
              首个 live daily run 成功后展示虚拟资金账户。
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : isLoading ? (
        <div className="grid min-h-[8.5rem] gap-x-6 gap-y-4 py-3 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, index) => (
            <div key={index} className="min-w-0 py-1">
              <Skeleton className="h-3 w-20 bg-muted/70" />
              <Skeleton className="mt-2 h-5 w-32 bg-muted" />
            </div>
          ))}
        </div>
      ) : isError || !account ? (
        <Empty className="py-8">
          <EmptyHeader>
            <EmptyTitle>虚拟资金账户加载失败</EmptyTitle>
            <EmptyDescription>
              当前组合尚未返回可展示的账户快照。
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="grid gap-x-6 gap-y-4 py-3 sm:grid-cols-2 lg:grid-cols-3">
          {metrics.map((metric) => (
            <div key={metric.label} className="min-w-0 py-1">
              <div className="truncate text-xs text-muted-foreground">
                {metric.label}
              </div>
              <div
                className={cn(
                  "mt-1 min-w-0 text-sm leading-tight font-medium break-words tabular-nums",
                  metric.tone === "up"
                    ? "text-[color:var(--portfolio-up)]"
                    : metric.tone === "down"
                      ? "text-[color:var(--portfolio-down)]"
                      : "text-foreground"
                )}
              >
                {metric.value}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  )
}

function StatementSection({
  isError,
  isLoading,
  isPendingFirstRun,
  offset,
  onNextPage,
  onPeriodChange,
  onPreviousPage,
  period,
  statement,
}: {
  isError: boolean
  isLoading: boolean
  isPendingFirstRun: boolean
  offset: number
  onNextPage: () => void
  onPeriodChange: (period: StrategyPortfolioStatementPeriodKey) => void
  onPreviousPage: () => void
  period: StrategyPortfolioStatementPeriodKey
  statement: StrategyPortfolioStatementResponse | null
}) {
  const operations = statement?.operations.items ?? []
  const summaryMetrics = statement
    ? [
        {
          label: "平均仓位",
          value: formatOptionalPercent(statement.summary.average_position_pct),
          tone: "neutral" as const,
        },
        {
          label: "交易股票数",
          value: formatInteger(statement.summary.traded_security_count),
          tone: "neutral" as const,
        },
        {
          label: "交易笔数",
          value: formatInteger(statement.summary.trade_count),
          tone: "neutral" as const,
        },
        {
          label: "交易成功率",
          value: formatOptionalPercent(statement.summary.trade_win_rate),
          tone: signedTone(statement.summary.trade_win_rate),
        },
        {
          label: "盈利股票数",
          value: formatInteger(statement.summary.winning_security_count),
          tone: "up" as const,
        },
        {
          label: "亏损股票数",
          value: formatInteger(statement.summary.losing_security_count),
          tone: "down" as const,
        },
        {
          label: "持股天数",
          value: `${formatInteger(statement.summary.holding_days)}天`,
          tone: "neutral" as const,
        },
      ]
    : []
  const pageStart = operations.length > 0 ? offset + 1 : 0
  const pageEnd = offset + operations.length

  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <div className="text-sm font-medium">对账单</div>
          {statement ? (
            <div className="mt-1 text-xs text-muted-foreground tabular-nums">
              {statement.period.start_date} 至 {statement.period.end_date}
            </div>
          ) : null}
        </div>
        <div className="flex h-8 max-w-full shrink-0 items-center overflow-x-auto rounded-md border border-border/70 p-0.5">
          {STATEMENT_PERIOD_OPTIONS.map((option) => {
            const selected = option.key === period

            return (
              <Button
                key={option.key}
                aria-pressed={selected}
                data-state={selected ? "selected" : "idle"}
                className="h-6 shrink-0 px-2 text-xs text-muted-foreground data-[state=selected]:bg-muted data-[state=selected]:text-foreground"
                size="sm"
                type="button"
                variant="ghost"
                onClick={() => onPeriodChange(option.key)}
              >
                {option.label}
              </Button>
            )
          })}
        </div>
      </div>

      {isPendingFirstRun ? (
        <Empty className="py-8">
          <EmptyHeader>
            <EmptyTitle>尚未产生对账单</EmptyTitle>
            <EmptyDescription>
              首个 live daily run 成功后展示账户盈亏和操作记录。
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : isLoading ? (
        <div className="grid min-h-[8.5rem] gap-x-6 gap-y-4 py-3 sm:grid-cols-2 lg:grid-cols-4">
          {Array.from({ length: 7 }).map((_, index) => (
            <div key={index} className="min-w-0 py-1">
              <Skeleton className="h-3 w-20 bg-muted/70" />
              <Skeleton className="mt-2 h-5 w-24 bg-muted" />
            </div>
          ))}
        </div>
      ) : isError || !statement ? (
        <Empty className="py-8">
          <EmptyHeader>
            <EmptyTitle>对账单加载失败</EmptyTitle>
            <EmptyDescription>
              当前组合尚未返回可展示的对账单数据。
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <>
          <div className="grid gap-x-6 gap-y-4 py-3 sm:grid-cols-2 lg:grid-cols-4">
            {summaryMetrics.map((metric) => (
              <div key={metric.label} className="min-w-0 py-1">
                <div className="truncate text-xs text-muted-foreground">
                  {metric.label}
                </div>
                <div
                  className={cn(
                    "mt-1 min-w-0 text-sm leading-tight font-medium break-words tabular-nums",
                    metric.tone === "up"
                      ? "text-[color:var(--portfolio-up)]"
                      : metric.tone === "down"
                        ? "text-[color:var(--portfolio-down)]"
                        : "text-foreground"
                  )}
                >
                  {metric.value}
                </div>
              </div>
            ))}
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="text-sm font-medium">操作记录</div>
            <div className="text-xs text-muted-foreground tabular-nums">
              {pageStart}-{pageEnd}
            </div>
          </div>

          {operations.length > 0 ? (
            <div className="[scrollbar-width:thin] overflow-x-auto">
              <Table className="min-w-[980px] table-fixed text-xs leading-snug [&_td]:overflow-hidden [&_th]:overflow-hidden">
                <TableHeader>
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="h-6 w-[9%] px-1">调仓日</TableHead>
                    <TableHead className="h-6 w-[7%] px-1">方向</TableHead>
                    <TableHead className="h-6 w-[17%] px-1">个股</TableHead>
                    <TableHead className="h-6 w-[9%] px-1 text-right">
                      价格
                    </TableHead>
                    <TableHead className="h-6 w-[9%] px-1 text-right">
                      数量
                    </TableHead>
                    <TableHead className="h-6 w-[7%] px-1 text-right">
                      手数
                    </TableHead>
                    <TableHead className="h-6 w-[11%] px-1 text-right">
                      金额
                    </TableHead>
                    <TableHead className="h-6 w-[10%] px-1 text-right">
                      费用
                    </TableHead>
                    <TableHead className="h-6 w-[10%] px-1 text-right">
                      持仓余额
                    </TableHead>
                    <TableHead className="h-6 w-[11%] px-1 text-right">
                      实现盈亏
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {operations.map((operation) => (
                    <TableRow key={operation.portfolio_trade_id}>
                      <TableCell className="px-1 py-0.5 tabular-nums">
                        {operation.trade_date}
                      </TableCell>
                      <TableCell
                        className={cn(
                          "px-1 py-0.5 font-medium",
                          operation.side === "sell"
                            ? "text-[color:var(--portfolio-down)]"
                            : "text-[color:var(--portfolio-up)]"
                        )}
                      >
                        {formatStatementSide(operation.side)}
                      </TableCell>
                      <TableCell className="px-1 py-0.5">
                        <div className="grid min-w-0 grid-cols-[4.25em_minmax(0,1fr)] items-center gap-1">
                          <span className="truncate font-medium">
                            {operation.security_name || operation.security_code}
                          </span>
                          <span className="truncate text-muted-foreground tabular-nums">
                            {operation.security_code}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell className="px-1 py-0.5 text-right tabular-nums">
                        {formatAccountCurrency(operation.execution_price)}
                      </TableCell>
                      <TableCell className="px-1 py-0.5 text-right tabular-nums">
                        {formatShareQuantity(operation.quantity)}
                      </TableCell>
                      <TableCell className="px-1 py-0.5 text-right tabular-nums">
                        {formatLotCount(operation.lot_count)}
                      </TableCell>
                      <TableCell className="px-1 py-0.5 text-right tabular-nums">
                        {formatAccountCurrency(operation.gross_amount)}
                      </TableCell>
                      <TableCell className="px-1 py-0.5 text-right tabular-nums">
                        {formatAccountCurrency(operation.total_fee)}
                      </TableCell>
                      <TableCell className="px-1 py-0.5 text-right tabular-nums">
                        {formatShareQuantity(
                          operation.position_balance_quantity
                        )}
                      </TableCell>
                      <TableCell
                        className={cn(
                          "px-1 py-0.5 text-right font-medium tabular-nums",
                          getSignedValueClassName(
                            formatAccountOptionalSignedCurrency(
                              operation.realized_pnl
                            )
                          )
                        )}
                      >
                        {formatAccountOptionalSignedCurrency(
                          operation.realized_pnl
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          ) : (
            <Empty className="border border-dashed border-border/70 py-8">
              <EmptyHeader>
                <EmptyTitle>暂无操作记录</EmptyTitle>
                <EmptyDescription>
                  当前区间内没有买入或卖出记录。
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}

          <div className="flex items-center justify-end gap-2">
            <Button
              disabled={offset <= 0}
              size="sm"
              type="button"
              variant="outline"
              onClick={onPreviousPage}
            >
              上一页
            </Button>
            <Button
              disabled={!statement.operations.has_more}
              size="sm"
              type="button"
              variant="outline"
              onClick={onNextPage}
            >
              下一页
            </Button>
          </div>
        </>
      )}
    </section>
  )
}

function SignalCountChart({
  onTimeSelect,
  points,
  selectedTime,
}: {
  onTimeSelect: (time: string) => void
  points: { time: string; value: number }[]
  selectedTime?: string
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const container = containerRef.current

    if (!container) {
      return
    }

    const chart = createChart(container, {
      width: container.clientWidth,
      height: container.clientHeight,
      layout: {
        background: { color: "transparent" },
        textColor: "rgba(99, 95, 89, 0.78)",
        attributionLogo: false,
      },
      localization: {
        timeFormatter: formatChartDate,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { color: "rgba(120, 114, 108, 0.12)" },
      },
      crosshair: {
        vertLine: { color: "rgba(120, 114, 108, 0.20)" },
        horzLine: { color: "rgba(120, 114, 108, 0.20)" },
      },
      rightPriceScale: {
        borderVisible: false,
      },
      leftPriceScale: {
        visible: false,
      },
      timeScale: {
        borderVisible: false,
        timeVisible: false,
        tickMarkFormatter: formatChartAxisDate,
        tickMarkMaxCharacterLength: 7,
      },
      handleScroll: false,
      handleScale: false,
    })

    const signalSeries = chart.addSeries(LineSeries, {
      color: "#2b2622",
      lineWidth: 2,
      lastValueVisible: false,
      priceLineVisible: false,
    })

    signalSeries.setData(points)
    chart.timeScale().fitContent()

    const selectedPoint = points.find((point) => point.time === selectedTime)

    if (selectedPoint) {
      chart.setCrosshairPosition(
        selectedPoint.value,
        selectedPoint.time,
        signalSeries
      )
    }

    function handleClick(param: { time?: unknown }) {
      if (typeof param.time !== "string") {
        return
      }

      onTimeSelect(param.time)
    }

    chart.subscribeClick(handleClick)

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0]

      if (!entry) {
        return
      }

      chart.applyOptions({
        height: entry.contentRect.height,
        width: entry.contentRect.width,
      })
    })

    resizeObserver.observe(container)

    return () => {
      chart.unsubscribeClick(handleClick)
      resizeObserver.disconnect()
      chart.remove()
    }
  }, [onTimeSelect, points, selectedTime])

  return <div ref={containerRef} className="h-56 w-full cursor-crosshair" />
}

function buildDetailPortfolioView(
  record: StrategyPortfolioRecord,
  navPoints: StrategyBacktestNavPoint[],
  performance: StrategyPortfolioPerformanceView | null
) {
  const curve = navPoints
    .filter((point) => typeof point.benchmark_nav === "number")
    .map((point) => ({
      benchmark: point.benchmark_nav as number,
      nav: point.strategy_nav,
      time: point.trade_date,
    }))
  const latestPoint = curve.at(-1)

  return {
    id: record.strategy_portfolio_id,
    name: record.name,
    startDate: record.live_start_date,
    simulationDays: latestPoint
      ? daysBetween(record.live_start_date, latestPoint.time)
      : 0,
    latestNav: latestPoint?.nav ?? null,
    dailyWinRate: readDailyWinRate(performance),
    returns: [
      metricFromPerformance("持仓收益", performance, "holding_period_return"),
      metricFromPerformance("年化收益", performance, "annualized_return"),
    ],
    risk: [
      metricFromPerformance("最大回撤", performance, "max_drawdown", "down"),
      metricFromPerformance(
        "年化波动率",
        performance,
        "annualized_volatility",
        "neutral"
      ),
      metricFromPerformance(
        "下行波动率",
        performance,
        "downside_deviation",
        "neutral"
      ),
    ],
    efficiency: [
      ratioMetricFromPerformance("Sharpe Ratio", performance, "sharpe_ratio"),
      ratioMetricFromPerformance("Sortino Ratio", performance, "sortino_ratio"),
      ratioMetricFromPerformance("Calmar Ratio", performance, "calmar_ratio"),
      ratioMetricFromPerformance("Treynor Ratio", performance, "treynor_ratio"),
    ],
    relative: [
      metricFromPerformance("Alpha", performance, "alpha", "neutral"),
      ratioMetricFromPerformance("Beta", performance, "beta"),
      ratioMetricFromPerformance(
        "Information Ratio",
        performance,
        "information_ratio"
      ),
    ],
    benchmarkSecurityCode: record.benchmark_security_code,
    curve,
  }
}

function metricFromPerformance(
  label: string,
  performance: StrategyPortfolioPerformanceView | null,
  key: string,
  fallbackTone?: Metric["tone"]
): Metric {
  const value = readPerformanceMetric(performance, key)

  return {
    label,
    value,
    kind: "percent",
    tone:
      fallbackTone ??
      (typeof value === "number" ? (value >= 0 ? "up" : "down") : "neutral"),
  }
}

function ratioMetricFromPerformance(
  label: string,
  performance: StrategyPortfolioPerformanceView | null,
  key: string
): Metric {
  return {
    label,
    value: readPerformanceMetric(performance, key),
    kind: "ratio",
    tone: "neutral",
  }
}

function readPerformanceMetric(
  performance: StrategyPortfolioPerformanceView | null,
  key: string
) {
  const value = performance?.metric[key]

  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function readDailyWinRate(
  performance: StrategyPortfolioPerformanceView | null
) {
  const value = performance?.daily_win_rate.value

  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function buildSignalPoolsFromApi(
  timeline: StrategyPortfolioSignalTimelinePoint[],
  selectedDate: string | null,
  signals: StrategyBacktestTargetRecord[],
  pendingSignals: StrategyPortfolioDashboardCard["pending_buy_signals"]
): SignalPool[] {
  const points = timeline.map((point) => ({
    date: point.trade_date,
    signalCount: point.signal_count ?? point.target_count,
    stocks:
      point.trade_date === selectedDate
        ? pendingSignals.length > 0
          ? pendingSignals
              .filter((signal) => signal.signal_date === point.trade_date)
              .map(mapPendingBuySignal)
          : signals
              .map(mapApiSignalTarget)
              .filter((stock): stock is SignalStock => stock !== null)
        : [],
  }))

  if (selectedDate && !points.some((point) => point.date === selectedDate)) {
    const selectedStocks = signals
      .map(mapApiSignalTarget)
      .filter((stock): stock is SignalStock => stock !== null)

    points.push({
      date: selectedDate,
      signalCount: selectedStocks.length,
      stocks: selectedStocks,
    })
  }

  return points.sort((left, right) => left.date.localeCompare(right.date))
}

function mapApiSignalTarget(
  target: StrategyBacktestTargetRecord
): SignalStock | null {
  if (typeof target.source_score !== "number") {
    return null
  }

  return {
    code: target.security_code,
    name: target.security_name?.trim() || target.security_code,
    score: target.source_score,
    scoreItems: [],
  }
}

function mapPendingBuySignal(
  signal: StrategyPortfolioDashboardCard["pending_buy_signals"][number]
): SignalStock {
  return {
    code: signal.code,
    executionDate: signal.execution_date,
    name: signal.name.trim() || signal.code,
    rank: signal.rank,
    score: signal.score,
    scoreItems: [],
    signalDate: signal.signal_date,
  }
}

function mapApiRebalanceRecord(
  record: StrategyBacktestRebalanceRecord
): DetailRebalanceRecord {
  return {
    date: record.trade_date,
    trades: record.rows.map((row) => ({
      changePercent: formatOptionalSignedPercent(row.change_pct),
      contribution: formatOptionalSignedPercent(row.contribution_pct),
      costPrice: formatOptionalCurrency(row.cost_price),
      currentPrice: formatOptionalCurrency(row.current_price),
      direction: row.direction,
      holdingDays:
        typeof row.holding_days === "number" ? `${row.holding_days}天` : "—",
      rebalanceReason: row.reason ?? "",
      securityCode: row.security_code,
      securityName: row.security_name?.trim() || row.security_code,
    })),
  }
}

function daysBetween(startDate: string, endDate: string) {
  const start = new Date(`${startDate}T00:00:00Z`).getTime()
  const end = new Date(`${endDate}T00:00:00Z`).getTime()

  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) {
    return 0
  }

  return Math.floor((end - start) / 86_400_000) + 1
}

function buildReturnMetrics(
  metrics: Metric[],
  excessReturn: number | null,
  dailyWinRate: number | null
): Metric[] {
  const excessMetric: Metric = {
    label: "超额收益",
    value: excessReturn,
    kind: "percent",
    tone:
      typeof excessReturn === "number"
        ? excessReturn >= 0
          ? "up"
          : "down"
        : "neutral",
  }
  const winRateMetric: Metric = {
    label: "日胜率",
    value: dailyWinRate,
    kind: "percent",
    tone: "neutral",
  }

  return [metrics[0], excessMetric, ...metrics.slice(1), winRateMetric].filter(
    (metric): metric is Metric => Boolean(metric)
  )
}

function formatSignalScoreItems(scoreItems: SignalScoreItem[]) {
  if (scoreItems.length === 0) {
    return "-"
  }

  return scoreItems
    .map((item) => `${item.label} ${item.score.toFixed(1)}`)
    .join(" / ")
}

function formatSignalDetail(stock: SignalStock) {
  if (stock.signalDate && stock.executionDate) {
    return `${stock.signalDate} -> ${stock.executionDate}`
  }

  return formatSignalScoreItems(stock.scoreItems)
}

function isPortfolioPendingFirstRunError(error: unknown) {
  return (
    error instanceof ApiError &&
    error.errorType === "portfolio_pending_first_run"
  )
}

function buildRebalanceTradeSections(trades: DetailTrade[]) {
  return (["buy", "hold", "sell"] as const).map((direction) => ({
    direction,
    trades: trades.filter((trade) => trade.direction === direction),
  }))
}

function getPositionCount(trades: DetailTrade[]) {
  return trades.filter((trade) => trade.direction !== "sell").length
}

function formatTradeDirection(direction: DetailTradeDirection) {
  if (direction === "buy") {
    return "调入"
  }

  if (direction === "hold") {
    return "持有"
  }

  return "调出"
}

function formatCompactDate(date: string) {
  const [, month, day] = date.split("-")

  return `${month}/${day}`
}

function formatChartDate(time: unknown) {
  const date = chartTimeToDateKey(time)

  return date ? date.replaceAll("-", "/") : String(time)
}

function formatChartAxisDate(time: unknown, tickMarkType: TickMarkType) {
  const date = chartTimeToDateKey(time)

  if (!date) {
    return String(time)
  }

  const [year, month, day] = date.split("-")

  if (tickMarkType === TickMarkType.Year) {
    return year
  }

  if (tickMarkType === TickMarkType.Month) {
    return `${year}/${month}`
  }

  return `${month}/${day}`
}

function chartTimeToDateKey(time: unknown) {
  if (typeof time === "string") {
    return time
  }

  if (
    time &&
    typeof time === "object" &&
    "year" in time &&
    "month" in time &&
    "day" in time
  ) {
    const value = time as { day: number; month: number; year: number }
    return `${value.year}-${String(value.month).padStart(2, "0")}-${String(
      value.day
    ).padStart(2, "0")}`
  }

  return null
}

function formatCurrency(value: number) {
  return `¥${value.toFixed(2)}`
}

function formatAccountCurrency(value: number) {
  return value.toFixed(2)
}

function formatInteger(value: number) {
  return Math.trunc(value).toLocaleString("zh-CN")
}

function formatShareQuantity(value: number) {
  return Math.trunc(value).toLocaleString("zh-CN")
}

function formatLotCount(value: number) {
  return Number.isInteger(value) ? value.toFixed(0) : value.toFixed(2)
}

function formatAccountSignedCurrency(value: number) {
  const sign = value > 0 ? "+" : value < 0 ? "-" : ""
  const absolute = Math.abs(value)

  return `${sign}${absolute.toFixed(2)}`
}

function formatOptionalPercent(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? `${(value * 100).toFixed(2)}%`
    : "--"
}

function formatOptionalCurrency(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? formatCurrency(value)
    : "—"
}

function formatAccountOptionalSignedCurrency(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? formatAccountSignedCurrency(value)
    : "--"
}

function formatSignedPercent(value: number) {
  const sign = value > 0 ? "+" : ""

  return `${sign}${(value * 100).toFixed(2)}%`
}

function formatOptionalSignedPercent(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? formatSignedPercent(value)
    : "—"
}

function formatAccountOptionalSignedPercent(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value)
    ? formatSignedPercent(value)
    : "--"
}

function formatStatementSide(side: "buy" | "sell") {
  return side === "sell" ? "卖出" : "买入"
}

function getScoreBadgeVariant(score: number) {
  if (score >= 85) {
    return "default"
  }

  if (score >= 70) {
    return "secondary"
  }

  return "outline"
}

function getSignedValueClassName(value: string) {
  if (value.startsWith("+")) {
    return "text-[color:var(--portfolio-up)]"
  }

  if (value.startsWith("-")) {
    return "text-[color:var(--portfolio-down)]"
  }

  return "text-foreground"
}

function signedTone(value: number | null | undefined) {
  if (typeof value !== "number" || !Number.isFinite(value) || value === 0) {
    return "neutral" as const
  }

  return value > 0 ? ("up" as const) : ("down" as const)
}

export { StrategyDetailPage }
