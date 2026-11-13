use strum::Display;

use crate::{ShellState, history::HistHandling, pipeline::RunningPipeline};

pub struct Job {
    number: usize,
    pipeline: RunningPipeline,
    text: String,
    status: JobStatus,
}

#[derive(Copy, Clone, PartialEq)]
enum JobStatus {
    Running,
    Done,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match self {
            JobStatus::Running => "Running",
            JobStatus::Done => "Done",
        })
    }
}

pub struct JobsList {
    jobs: Vec<Job>,
}

impl JobsList {
    pub fn new() -> Self {
        JobsList { jobs: Vec::new() }
    }
    pub fn add(&mut self, pipeline: RunningPipeline, text: String) {
        self.jobs.push(Job {
            number: self.jobs.last().map_or(1, |j| j.number + 1),
            pipeline,
            text: text,
            status: JobStatus::Running,
        });
        println!(
            "[{}] {}",
            self.jobs.last().expect("Last is never empty").number,
            self.jobs
                .last()
                .expect("Last is never empty")
                .pipeline
                .get_pid()
        )
    }

    /// Returns a sorted list of Jobs, but w/o running pipelines
    /// Handy to show old and new Jobs
    pub fn combine_all_jobs(&self, other: &Self) -> Self {
        let mut out = JobsList::new();
        for job in &self.jobs {
            let mut new_text = job.text.clone();
            if job.status == JobStatus::Done {
                new_text = new_text.trim_end_matches('&').to_string();
            }
            out.jobs.push(Job {
                number: job.number,
                pipeline: RunningPipeline::new(),
                text: job.text.clone(),
                status: job.status,
            })
        }
        for job in &other.jobs {
            let mut new_text = job.text.clone();
            if job.status == JobStatus::Done {
                new_text = new_text.trim_end_matches('&').to_string();
            }
            out.jobs.push(Job {
                number: job.number,
                pipeline: RunningPipeline::new(),
                text: new_text,
                status: job.status,
            })
        }
        out.jobs.sort_by_key(|job| job.number);
        out
    }

    pub fn show(&self) -> String {
        let most_recent_job_no = self.jobs.last().and_then(|job| Some(job.number));
        let Some(most_recent_job_no) = most_recent_job_no else {
            return "".to_string();
        };

        let second_recent_job_no = self.jobs.iter().rev().nth(1).map(|job| job.number);

        let mut out = String::new();

        for job in &self.jobs {
            out.push_str(
                &format!(
                    "[{num}]{recent}  {status:width$}{text}\n",
                    num = job.number,
                    recent = format_recent(job.number, most_recent_job_no, second_recent_job_no),
                    status = job.status.to_string(),
                    text = job.text,
                    width = 24,
                )[..],
            );
        }
        out
    }

    pub fn reap_finished(&mut self) -> JobsList {
        let mut done_jobs = JobsList::new();
        for job in self.jobs.extract_if(.., |job| job.pipeline.try_wait()) {
            //wait is blocking, but we are sure that last child has already ended
            job.pipeline.wait();
            done_jobs.jobs.push(Job {
                number: job.number,
                pipeline: RunningPipeline::new(),
                text: job.text,
                status: JobStatus::Done,
            });
        }
        done_jobs
    }
}

fn format_recent(job_no: usize, most_recent: usize, sec_recent: Option<usize>) -> &'static str {
    if job_no == most_recent {
        "+"
    } else if sec_recent == Some(job_no) {
        "-"
    } else {
        " "
    }
}
