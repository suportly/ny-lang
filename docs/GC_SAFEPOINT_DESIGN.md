# Tabela de transições — handshake STW

## Invariante central
    g_running == |{ t : t.state == RUNNING }|
Verificável a qualquer momento sob g_stw_mutex.

## Invariante de segurança
    Durante mark+sweep: g_running == 0  E  stw_active == 1
    (nenhum mutador rodando código Ny)

## Invariante de progresso
    Toda thread em PARKED esperando g_resume será acordada por stw_end.
    Todo coletor esperando g_all_parked será acordado quando g_running→0.

## Estados
    OUTSIDE : não participa (sem roots ainda, ou já saiu)
    RUNNING : executando código Ny, conta em g_running
    PARKED  : em safepoint / bloqueada / coletando, não conta

## Transições (todas sob g_stw_mutex)

| evento          | de       | para     | ação                                    |
|-----------------|----------|----------|------------------------------------------|
| join            | OUTSIDE  | RUNNING  | participants++; espera !stw; running++    |
| leave           | RUNNING  | OUTSIDE  | running--; participants--; bcast se 0     |
| leave           | PARKED   | OUTSIDE  | participants--                            |
| park (depth 0→1)| RUNNING  | PARKED   | running--; bcast se 0                     |
| park (depth n>0)| PARKED   | PARKED   | (nada — só depth++)                       |
| unpark(depth1→0)| PARKED   | RUNNING  | espera !stw; running++                    |
| safepoint       | RUNNING  | PARKED   | running--; bcast se 0; espera !stw;       |
|   (se stw)      |          | RUNNING  | running++                                 |
| safepoint       | PARKED   | PARKED   | no-op (já parkeada)                       |
| collect_begin   | RUNNING  | PARKED   | running--; bcast se 0; espera !stw;       |
|                 |          |          | stw=1; espera running==0                  |
| collect_begin   | PARKED   | PARKED   | espera !stw; stw=1; espera running==0     |
| collect_end     | PARKED   | RUNNING  | stw=0; bcast resume; running++            |
|   (se depth>0)  | PARKED   | PARKED   | stw=0; bcast resume  (unpark restaura)    |

## Casos que quebraram antes (agora explícitos)
1. join durante stw pendente        → entra esperando, não como RUNNING
2. collect enquanto outro coleta    → fica PARKED enquanto espera a vez
3. bloqueio em mutex ≠ safepoint    → coletor não segura g_heap_mutex
4. unpark segurando lock alheio     → unpark sempre após o unlock do site
5. park no-op + unpark real         → depth por thread, não flag derivada

## O que ainda não estava enumerado (suspeitos do bug restante)
A. collect_end quando a thread entrou em collect já PARKED (depth>0)
   → quem restaura running? unpark. Mas e se stw_end também restaurar?
B. safepoint chamado por thread PARKED (dentro de park bracket)
   → deve ser no-op; se decrementar, running fica negativo
C. leave (destrutor) enquanto PARKED com depth>0
   → participants-- mas running já estava decrementado
