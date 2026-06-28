用例策略

过滤条件：

1. KDJ的 J < 13
2. amplitude  4%
3. -2%   < pct_change < + 2%
4. volume < prev_volume * 0.8
5. ema2_10 > avg_ma_14_28_57_114
6. close_down_streak_days < 4
7. forward_close_price > price_avg_ma_3_6_12_24
8. price_ma_60 > price_ma_114  and  price_ma_114 > price_ma_250

得分条件
1. J < -15 (+25) or J < -10 (+15)
2. volume < volume_ma_5 * 0.6 (+20)
4. price_ma_20 < forward_close_price < price_ma_60 (+15)
5. n_structure_20_second_low_ratio > 1 (+15)
6. forward_close_price < boll_dn_20_2 (+15)
7. rsi_6 < 25 (+5)


