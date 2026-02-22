# 性能测试

## 测试数据

1. haystack. 16B * 1024 random generated string
2. needled: size ∈ {1, 2, 3, 4, 6, 8}

## 结果

| 测试用例 (Needle) | 算法 (Algorithm) | 平均耗时 (Time) | 平均吞吐量 (Throughput) |
| ----------------- | ---------------- | --------------- | ----------------------- |
| n1                | shufti           | 16.837 ns       | 906.29 GiB/s            |
| n1                | naive            | 142.92 ns       | 106.76 GiB/s            |
| n1                | memchr           | 6.2425 ns       | 2444.3 GiB/s            |
| n2                | shufti           | 16.885 ns       | 903.67 GiB/s            |
| n2                | naive            | 205.56 ns       | 74.232 GiB/s            |
| n2                | memchr           | 7.1109 ns       | 2145.8 GiB/s            |
| n3                | shufti           | 16.919 ns       | 901.90 GiB/s            |
| n3                | naive            | 393.86 ns       | 38.741 GiB/s            |
| n3                | memchr           | 9.9295 ns       | 1536.7 GiB/s            |
| n4                | shufti           | 16.906 ns       | 902.55 GiB/s            |
| n4                | naive            | 470.86 ns       | 32.406 GiB/s            |
| n4                | memchr           | 364.09 ns       | 41.909 GiB/s            |
| n6                | shufti           | 16.818 ns       | 907.28 GiB/s            |
| n6                | naive            | 626.01 ns       | 24.375 GiB/s            |
| n6                | memchr           | 518.75 ns       | 29.414 GiB/s            |
| n8                | shufti           | 16.868 ns       | 904.58 GiB/s            |
| n8                | naive            | 787.35 ns       | 19.380 GiB/s            |
| n8                | memchr           | 920.37 ns       | 16.579 GiB/s            |