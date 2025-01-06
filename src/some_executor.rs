/*!
Implements the some_executor traits for the main thread executor
*/

use std::any::Any;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use some_executor::observer::{FinishedObservation, Observer, ObserverNotified};
use some_executor::SomeLocalExecutor;
use some_executor::task::Task;
use crate::executor::{on_main_thread_async, on_main_thread_async_submit};

pub struct MainThreadExecutor {

}

//Since this executor is globally-scoped, we use 'static for the lifetime
impl SomeLocalExecutor<'static> for MainThreadExecutor {
    type ExecutorNotifier = Infallible;

    fn spawn_local<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> impl Observer<Value=F::Output>
    where
        Self: Sized,
        F: 'static,
        <F as Future>::Output: Unpin,
        <F as Future>::Output: 'static
    {
        let (s,o) = task.spawn_local(self);
        on_main_thread_async_submit(async { s.into_future().await;});
        o
    }

    fn spawn_local_async<F: Future, Notifier: ObserverNotified<F::Output>>(&mut self, task: Task<F, Notifier>) -> impl Future<Output=impl Observer<Value=F::Output>>
    where
        Self: Sized,
        F: 'static,
        <F as Future>::Output: Unpin,
        <F as Future>::Output: 'static,
    {   let (s,o) = task.spawn_local(self);
        async move {
            on_main_thread_async_submit(async{s.into_future().await;});
            o
        }

    }

    fn spawn_local_objsafe(&mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Box<(dyn Observer<Output = FinishedObservation<Box<(dyn Any + 'static)>>, Value = Box<(dyn Any + 'static)>> + 'static)> {
        let (s,o) = task.spawn_local_objsafe(self);
        on_main_thread_async_submit(async { s.into_future().await;});
        Box::new(o)
    }

    fn spawn_local_objsafe_async<'s>(&'s mut self, task: Task<Pin<Box<dyn Future<Output=Box<dyn Any>>>>, Box<dyn ObserverNotified<(dyn Any + 'static)>>>) -> Box<(dyn std::future::Future<Output = Box<(dyn Observer<Output = FinishedObservation<Box<(dyn Any + 'static)>>, Value = Box<(dyn Any + 'static)>> + 'static)>> + 's)> {
        Box::new(async {
            let (s,o) = task.spawn_local_objsafe(self);
            on_main_thread_async_submit(async { s.into_future().await;});
            Box::new(o) as Box<dyn Observer<Output = FinishedObservation<Box<dyn Any>>, Value = Box<dyn Any>>>
        })
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        None
    }
}