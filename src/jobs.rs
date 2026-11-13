use crate::{ShellState, history::HistHandling, pipeline::RunningPipeline};

pub struct Job {
    number: usize,
    pipeline: RunningPipeline,
    text: String,
    status: String,
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
            number: self.jobs.len() + 1,
            pipeline,
            text: text,
            status: "Running".to_string(),
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
    pub fn show(&self) -> String {
        let most_recent_job_no = self.jobs.last().and_then(|job| Some(job.number));

        let Some(most_recent_job_no) = most_recent_job_no else {
            return "".to_string();
        };
        let mut out = String::new();

        for job in &self.jobs {
            out.push_str(
                &format!(
                    "[{num}]{recent}  {status:width$}{text}\n",
                    num = job.number,
                    recent = if job.number == most_recent_job_no {
                        "+"
                    } else {
                        " "
                    },
                    status = job.status,
                    text = job.text,
                    width = 24,
                )[..],
            );
        }
        out
    }
    /*pub fn reap_finished(&mut self) {
        for job in &mut self.jobs {
            if job.pipeline.try_wait() {
                print
            }
        }
    }*/
}
