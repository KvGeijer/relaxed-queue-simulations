pub trait RelaxedFifo<T> {
    fn enqueue(&mut self, item: T);
    fn dequeue(&mut self) -> Option<T>;
}
