use std::collections::VecDeque;

use chrono::Utc;
use sysinfo::System;

use crate::model::SystemSnapshot;

const HISTORY_LENGTH: usize = 40;

pub struct SystemMonitor {
    system: System,
    cpu_history: VecDeque<f32>,
    memory_history: VecDeque<f32>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        system.refresh_memory();
        Self {
            system,
            cpu_history: VecDeque::with_capacity(HISTORY_LENGTH),
            memory_history: VecDeque::with_capacity(HISTORY_LENGTH),
        }
    }

    pub fn snapshot(&mut self) -> SystemSnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let cpu = self.system.global_cpu_usage().clamp(0.0, 100.0);
        let used_memory = self.system.used_memory();
        let total_memory = self.system.total_memory();
        let memory = if total_memory == 0 {
            0.0
        } else {
            (used_memory as f64 / total_memory as f64 * 100.0) as f32
        };

        push_sample(&mut self.cpu_history, cpu);
        push_sample(&mut self.memory_history, memory);

        let average_frequency = if self.system.cpus().is_empty() {
            None
        } else {
            Some(
                self.system
                    .cpus()
                    .iter()
                    .map(|cpu| cpu.frequency())
                    .sum::<u64>()
                    / self.system.cpus().len() as u64,
            )
        };

        SystemSnapshot {
            available: true,
            cpu_percent: cpu,
            memory_percent: memory,
            used_memory_bytes: used_memory,
            total_memory_bytes: total_memory,
            cpu_frequency_mhz: average_frequency,
            cpu_history: padded_history(&self.cpu_history),
            memory_history: padded_history(&self.memory_history),
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

fn push_sample(history: &mut VecDeque<f32>, value: f32) {
    if history.len() == HISTORY_LENGTH {
        history.pop_front();
    }
    history.push_back(value);
}

fn padded_history(history: &VecDeque<f32>) -> Vec<f32> {
    let first = history.front().copied().unwrap_or(0.0);
    let mut values = vec![first; HISTORY_LENGTH.saturating_sub(history.len())];
    values.extend(history.iter().copied());
    values
}
