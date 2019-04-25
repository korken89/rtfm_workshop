The dead pool design for EasyDMA


UARTE
 - tx DMA buffer, double buffered
 - rx DMA buffer, double buffered
 - events
    - TASKS_STARTRX     // Used only at startup
    - TASKS_STOPRX      // -
    - TASKS_STARTTX     // Used only if tx_idle
    - TASKS_STOPTX      // -
    - TASKS_FLUSHRX     // -
    - EVENTS_CTS        // -
    - EVENTS_NCTS       // -
    - EVENTS_RXDRDY     // -
    - EVENTS_ENDRX      // -
    - EVENTS_TXDRDY     // -
    - EVENTS_ENDTX      // -
    - EVENTS_ERROR      // -
    - EVENTS_RXTO       // -
    - EVENTS_RXSTARTED  // used to fill next RX
    - EVENTS_TXSTARTED  // used to fill next TX
    - EVENTS_TXSTOPPED  // -

RX functionality

1. on init
- allocate one DMA buffer b1 and TASKS_STARTRX
- dma_q.enqueue(b1);

- on EVENTS_RXSTARTED it will
- allocate one DMA buffer and b2 TASKS_STARTRX
- dma_q.enqueue(b2);


- on EVENTS_ENDRX it
- b = dma_q.denqueue();

RX {
    dma_q = Queue<<DMAPool>, U2> // hold a maximum of two buffers
}

TX funtcionality
1 on init
    - enable interrupts

2 on interrupt
//    - TXSTARTED -> enque in buffer if buffer not empty,
    - TXEND ->
        dequeue local outstanding,
        txc.enque() if not empty start transaction, put in local outstanding
    -
