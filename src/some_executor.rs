//SPDX-License-Identifier: MPL-2.0

/*!
Implements the some_executor traits for the main thread executor
*/

use crate::application::submit_to_main_thread;
use crate::executor::already_on_main_thread_submit;
use some_executor::observer::{FinishedObservation, Observer, ObserverNotified};
use some_executor::task::Task;
use some_executor::{BoxedSendObserverFuture, DynExecutor, LocalExecutorExt, ObjSafeTask, SomeExecutor, SomeExecutorExt, SomeLocalExecutor};
use std::any::Any;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct MainThreadExecutor {}

//Since this executor is globally-scoped, we use 'static for the lifetime
impl SomeLocalExecutor<'static> for MainThreadExecutor {
    type ExecutorNotifier = Infallible;

    fn spawn_local<F: Future, Notifier: ObserverNotified<F::Output>>(
        &mut self,
        task: Task<F, Notifier>,
    ) -> impl Observer<Value = F::Output>
    where
        Self: Sized,
        F: 'static,
        <F as Future>::Output: Unpin,
        <F as Future>::Output: 'static,
    {
        let (s, o) = task.spawn_local(self);
        already_on_main_thread_submit(async {
            s.into_future().await;
        });
        o
    }

    fn spawn_local_async<F: Future, Notifier: ObserverNotified<F::Output>>(
        &mut self,
        task: Task<F, Notifier>,
    ) -> impl Future<Output = impl Observer<Value = F::Output>>
    where
        Self: Sized,
        F: 'static,
        <F as Future>::Output: Unpin,
        <F as Future>::Output: 'static,
    {
        let (s, o) = task.spawn_local(self);
        async move {
            already_on_main_thread_submit(async {
                s.into_future().await;
            });
            o
        }
    }

    fn spawn_local_objsafe(
        &mut self,
        task: Task<
            Pin<Box<dyn Future<Output = Box<dyn Any>>>>,
            Box<dyn ObserverNotified<(dyn Any + 'static)>>,
        >,
    ) -> Box<
        (
            dyn Observer<
                    Output = FinishedObservation<Box<(dyn Any + 'static)>>,
                    Value = Box<(dyn Any + 'static)>,
                > + 'static
        ),
    > {
        let (s, o) = task.spawn_local_objsafe(self);
        already_on_main_thread_submit(async {
            s.into_future().await;
        });
        Box::new(o)
    }

    fn spawn_local_objsafe_async<'s>(
        &'s mut self,
        task: Task<
            Pin<Box<dyn Future<Output = Box<dyn Any>>>>,
            Box<dyn ObserverNotified<(dyn Any + 'static)>>,
        >,
    ) -> Box<
        (
            dyn std::future::Future<
                    Output = Box<
                        (
                            dyn Observer<
                                    Output = FinishedObservation<Box<(dyn Any + 'static)>>,
                                    Value = Box<(dyn Any + 'static)>,
                                > + 'static
                        ),
                    >,
                > + 's
        ),
    > {
        Box::new(async {
            let (s, o) = task.spawn_local_objsafe(self);
            already_on_main_thread_submit(async {
                s.into_future().await;
            });
            Box::new(o)
                as Box<
                    dyn Observer<Output = FinishedObservation<Box<dyn Any>>, Value = Box<dyn Any>>,
                >
        })
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        None
    }
}

impl LocalExecutorExt<'static> for MainThreadExecutor {}

impl SomeExecutor for MainThreadExecutor {
    type ExecutorNotifier = Infallible;

    fn spawn<F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(
        &mut self,
        task: Task<F, Notifier>,
    ) -> impl Observer<Value = F::Output>
    where
        Self: Sized,
        F::Output: Send + Unpin,
    {
        let (s, o) = task.spawn(self);
        submit_to_main_thread(|| {
            already_on_main_thread_submit(async {
                s.into_future().await;
            });
        });
        o
    }

    fn spawn_async<'s, F: Future + Send + 'static, Notifier: ObserverNotified<F::Output> + Send>(
        &'s mut self,
        task: Task<F, Notifier>,
    ) -> impl Future<Output = impl Observer<Value = F::Output>> + Send + 's
    where
        Self: Sized,
        F::Output: Send + Unpin,
    {
        async move {
            let (s, o) = task.spawn(self);
            submit_to_main_thread(|| {
                already_on_main_thread_submit(async {
                    s.into_future().await;
                });
            });
            o
        }
    }

    fn spawn_objsafe(&mut self, task: some_executor::ObjSafeTask) -> some_executor::BoxedSendObserver {
        let (s, o) = task.spawn_objsafe(self);
        submit_to_main_thread(|| {
            already_on_main_thread_submit(async {
                s.into_future().await;
            });
        });
        Box::new(o)
    }
    fn spawn_objsafe_async<'s>(&'s mut self, task: ObjSafeTask) -> BoxedSendObserverFuture<'s> {
        Box::new(async {
            let (s, o) = task.spawn_objsafe(self);
            submit_to_main_thread(|| {
                already_on_main_thread_submit(async {
                    s.into_future().await;
                });
            });
            Box::new(o)
                as Box<
                dyn Observer<
                    Value = Box<dyn Any + Send>,
                    Output = FinishedObservation<Box<dyn Any + Send>>,
                > + Send,
            >
        })
    }

    fn clone_box(&self) -> Box<DynExecutor> {
        Box::new(MainThreadExecutor {})
    }

    fn executor_notifier(&mut self) -> Option<Self::ExecutorNotifier> {
        None
    }
}

impl SomeExecutorExt for MainThreadExecutor {}
